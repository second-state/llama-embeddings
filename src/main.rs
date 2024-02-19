use clap::{crate_version, Arg, Command};
use endpoints::{
    chat::{
        ChatCompletionObject, ChatCompletionRequestBuilder, ChatCompletionRequestMessage,
        ChatCompletionRole,
    },
    embeddings::{EmbeddingObject, EmbeddingRequest, EmbeddingsResponse},
};
use qdrant::*;
use std::io::prelude::*;
use std::path::Path;
use std::{fs::File, vec};
use text_splitter::TextSplitter;
use tiktoken_rs::cl100k_base;

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), String> {
    let matches = Command::new("llama-chat")
        .version(crate_version!())
        .arg(
            Arg::new("file")
                .short('f')
                .long("file")
                .value_name("FILE")
                .help("File with the *.txt extension"),
        )
        .after_help("Example: wasmedge --dir .:. llama-embeddings.wasm --file test.txt\n")
        .get_matches();

    let file = matches.get_one::<String>("file").unwrap();

    // * load and chunk the text file
    let chunks = load_and_chunk_text(file)?;

    // * create embeddings
    let embeddings = llama_compute_embeddings(&chunks).await?;

    // * access qdrant db

    // create a Qdrant client
    let qdrant_client = qdrant::Qdrant::new();

    let collection_name = "my_test";
    let dim = embeddings[0].embedding.len();

    // create a collection
    qdrant_create_collection(&qdrant_client, collection_name, dim).await?;

    // create and upsert points
    qdrant_persist_embeddings(&qdrant_client, collection_name, &embeddings, &chunks).await?;

    // * compute embeddings for a query

    println!("\n\n=========== Tiny RAG Demo ===========\n");

    // let query_text = "What is the capital of France?";
    let query_text = "What is bitcoin?";
    println!("[You] {}\n\n", query_text);

    let query_embedding = llama_compute_embeddings(&[query_text.into()]).await?;
    let query_vector = query_embedding[0]
        .embedding
        .iter()
        .map(|x| *x as f32)
        .collect();

    // search for similar points
    let top = 3;
    let search_result =
        qdrant_search_similar_points(&qdrant_client, collection_name, query_vector, top).await?;

    // list the search results
    let mut context = Vec::new();
    for (i, point) in search_result.iter().enumerate() {
        println!("    * Point {}: score: {}", i, point.score);

        if let Some(payload) = &point.payload {
            if let Some(source) = payload.get("source") {
                println!("      Source: {}", source);
                context.push(source.to_string());
            }
        }
    }
    println!("\n\n");

    // * feed the query and the context to the model

    let answer = llama_chat(query_text, &context).await?;
    println!("[Bot]: {}", answer);

    Ok(())
}

fn load_and_chunk_text(file: &str) -> Result<Vec<String>, String> {
    println!("[+] Loading the text file ...");
    let file_path = Path::new(file);
    if !file_path.exists() {
        return Err("File does not exist".to_string());
    }
    if file_path.extension().is_none() || file_path.extension().unwrap() != "txt" {
        return Err("File is not a text file".to_string());
    }

    // read contents from a text file
    let mut file = File::open(file_path).expect("failed to open file");
    let mut text = String::new();
    file.read_to_string(&mut text).expect("failed to read file");

    // println!("File contents: {}", text);

    println!("[+] Chunking the text ...");
    let tokenizer = cl100k_base().unwrap();
    let max_tokens = 100;
    let splitter = TextSplitter::new(tokenizer).with_trim_chunks(true);

    let chunks = splitter
        .chunks(&text, max_tokens)
        .map(|s| s.to_string())
        .collect::<Vec<_>>();

    Ok(chunks)
}

async fn llama_compute_embeddings(chunks: &[String]) -> Result<Vec<EmbeddingObject>, String> {
    println!("[+] Creating embeddings for the chunks ...");
    let embedding_request = EmbeddingRequest {
        model: "dummy-embedding-model".to_string(),
        input: chunks.to_vec(),
        encoding_format: None,
        user: None,
    };
    let request_body = serde_json::to_value(&embedding_request).unwrap();

    // create a client
    let client = reqwest::Client::new();
    let embeddings = match client
        .post("http://localhost:8080/v1/embeddings")
        .header("accept", "application/json")
        .header("Content-Type", "application/json")
        .json(&request_body)
        .send()
        .await
    {
        Ok(response) => {
            let embedding_reponse: EmbeddingsResponse = response.json().await.unwrap();
            println!(
                "    * Number of embedding objects: {}",
                embedding_reponse.data.len()
            );

            embedding_reponse.data
        }
        Err(err) => {
            println!("Error: {}", err);
            return Err(err.to_string());
        }
    };

    Ok(embeddings)
}

async fn llama_chat(query: &str, context: &[String]) -> Result<String, String> {
    let mut context_s = String::new();
    for c in context.iter() {
        context_s.push_str(c);
        context_s.push_str("\n\n");
    }

    // create system message
    let content = format!("Use the following pieces of context to answer the user's question.\nIf you don't know the answer, just say that you don't know, don't try to make up an answer.\n----------------\n{}", context_s.trim_end());
    let system_message = ChatCompletionRequestMessage {
        role: ChatCompletionRole::System,
        content,
        name: None,
        function_call: None,
    };

    // create user message
    let user_message = ChatCompletionRequestMessage {
        role: ChatCompletionRole::User,
        content: query.to_string(),
        name: None,
        function_call: None,
    };

    let messages = vec![system_message, user_message];

    // create a chat completion request
    let chat_request = ChatCompletionRequestBuilder::new("llama-2-7b", messages).build();
    let request_body = serde_json::to_value(&chat_request).unwrap();

    // create a client
    let client = reqwest::Client::new();
    match client
        .post("http://localhost:8080/v1/chat/completions")
        .header("accept", "application/json")
        .header("Content-Type", "application/json")
        .json(&request_body)
        .send()
        .await
    {
        Ok(res) => {
            let chat_completion_object: ChatCompletionObject = match res.json().await {
                Ok(chat_completion_object) => chat_completion_object,
                Err(err) => {
                    println!("Error: {}", err);
                    return Err(err.to_string());
                }
            };

            Ok(chat_completion_object.choices[0].message.content.clone())
        }
        Err(err) => {
            println!("Error: {}", err);
            Err(err.to_string())
        }
    }
}

async fn qdrant_persist_embeddings(
    qdrant_client: &qdrant::Qdrant,
    collection_name: impl AsRef<str>,
    embeddings: &[EmbeddingObject],
    chunks: &[String],
) -> Result<(), String> {
    println!("[+] Creating points to save embeddings ...");
    let mut points = Vec::<Point>::new();
    for embedding in embeddings {
        // convert the embedding to a vector
        let vector: Vec<_> = embedding.embedding.iter().map(|x| *x as f32).collect();

        // create a payload
        let payload = serde_json::json!({"source": chunks[embedding.index as usize]})
            .as_object()
            .map(|m| m.to_owned());

        // create a point
        let p = Point {
            id: PointId::Num(embedding.index),
            vector,
            payload,
        };

        points.push(p);
    }
    // let dim = points[0].vector.len();

    // // create a Qdrant client
    // let qdrant_client = qdrant::Qdrant::new();

    // // Create a collection with `dim`-dimensional vectors
    // println!("[+] Creating a collection ...");
    // // let collection_name = "my_test";
    // println!("    * Collection name: {}", collection_name.as_ref());
    // println!("    * Dimension: {}", dim);
    // if let Err(err) = qdrant_client
    //     .create_collection(collection_name.as_ref(), dim as u32)
    //     .await
    // {
    //     println!("Failed to create collection. {}", err.to_string());
    //     return Err(err.to_string());
    // }

    // upsert points
    println!("[+] Upserting points ...");
    if let Err(err) = qdrant_client
        .upsert_points(collection_name.as_ref(), points)
        .await
    {
        println!("Failed to upsert points. {}", err.to_string());
        return Err(err.to_string());
    }

    Ok(())
}

async fn qdrant_create_collection(
    qdrant_client: &qdrant::Qdrant,
    collection_name: impl AsRef<str>,
    dim: usize,
) -> Result<(), String> {
    println!("[+] Creating a collection ...");
    // let collection_name = "my_test";
    println!("    * Collection name: {}", collection_name.as_ref());
    println!("    * Dimension: {}", dim);
    if let Err(err) = qdrant_client
        .create_collection(collection_name.as_ref(), dim as u32)
        .await
    {
        println!("Failed to create collection. {}", err.to_string());
        return Err(err.to_string());
    }

    Ok(())
}

async fn qdrant_search_similar_points(
    qdrant_client: &qdrant::Qdrant,
    collection_name: impl AsRef<str>,
    query_vector: Vec<f32>,
    limit: usize,
) -> Result<Vec<qdrant::ScoredPoint>, String> {
    println!("[+] Searching for similar points ...");
    let search_result = qdrant_client
        .search_points(collection_name.as_ref(), query_vector, limit as u64)
        .await;

    Ok(search_result)
}
