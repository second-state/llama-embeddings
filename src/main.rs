use clap::{crate_version, Arg, Command};
use endpoints::embeddings::{EmbeddingRequest, EmbeddingsResponse};
use qdrant::*;
use std::fs::File;
use std::io::prelude::*;
use std::path::Path;

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

    println!("[+] Reading text ...");

    let file = matches.get_one::<String>("file").unwrap().to_string();
    let file_path = Path::new(&file);
    if !file_path.exists() {
        println!("File {} does not exist", file_path.display());
        return Err("File does not exist".to_string());
    }
    if file_path.extension().is_none() || file_path.extension().unwrap() != "txt" {
        println!("File {} is not a text file", file_path.display());
        return Err("File is not a text file".to_string());
    }

    // read contents from a text file
    let mut file = File::open(file_path).expect("failed to open file");
    let mut text = String::new();
    file.read_to_string(&mut text).expect("failed to read file");

    // println!("File contents: {}", text);

    // * split text into chunks

    println!("[+] Chunking the text ...");
    let tokenizer = cl100k_base().unwrap();
    let max_tokens = 100;
    let splitter = TextSplitter::new(tokenizer).with_trim_chunks(true);

    let chunks = splitter.chunks(&text, max_tokens).collect::<Vec<_>>();

    // for chunk in chunks.iter() {
    //     println!("\nlen: {}, contents: {}\n", chunk.len(), chunk);
    // }

    // * create embeddings

    println!("[+] Creating embeddings for the chunks ...");
    let input = chunks.iter().map(|x| x.to_string()).collect();
    let embedding_request = EmbeddingRequest {
        model: "dummy-embedding-model".to_string(),
        input,
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

    // * access qdrant db

    println!("[+] Creating points to save embeddings ...");
    let mut points = Vec::<Point>::new();
    for embedding in embeddings.iter() {
        // convert the embedding to a vector
        let vector: Vec<_> = embedding.embedding.iter().map(|x| *x as f32).collect();

        // create a payload
        let payload =
            serde_json::json!({"source": &embedding_request.input[embedding.index as usize]})
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
    let dim = points[0].vector.len();

    // create a Qdrant client
    let qdrant_client = qdrant::Qdrant::new();

    // // Delete the collection if it exists
    // match qdrant_client.delete_collection_api(collection_name).await {
    //     Ok(_) => {
    //         println!("Collection deleted");
    //     }
    //     Err(err) => {
    //         println!("{}", err.to_string());
    //     }
    // }

    // Create a collection with `dim`-dimensional vectors
    println!("[+] Creating a collection ...");
    let collection_name = "my_test";
    println!("    * Collection name: {}", collection_name);
    println!("    * Dimension: {}", dim);
    if let Err(err) = qdrant_client
        .create_collection(collection_name, dim as u32)
        .await
    {
        println!("Failed to create collection. {}", err.to_string());
        return Err(err.to_string());
    }

    // upsert points
    println!("[+] Upserting points ...");
    if let Err(err) = qdrant_client.upsert_points(collection_name, points).await {
        println!("Failed to upsert points. {}", err.to_string());
        return Err(err.to_string());
    }

    // println!(
    //     "The collection size is {}",
    //     qdrant_client.collection_info(collection_name).await
    // );

    // let p = client.get_point("my_test", 0).await;
    // println!("The second point is {:?}", p);

    // let ps = client.get_points("my_test", vec![1, 2, 3, 4, 5, 6]).await;
    // println!("The 1-6 points are {:?}", ps);

    // let q = vec![0.2, 0.1, 0.9, 0.7];
    // let r = client.search_points("my_test", q, 2).await;
    // println!("Search result points are {:?}", r);

    // match client.delete_points("my_test", vec![0]).await {
    //     Ok(_) => {
    //         println!("Point deleted");
    //     }
    //     Err(err) => {
    //         println!("Error: {}", err);
    //         return Err(err.to_string());
    //     }
    // }

    // println!(
    //     "The collection size is {}",
    //     client.collection_info("my_test").await
    // );

    // let q = vec![0.2, 0.1, 0.9, 0.7];
    // let r = client.search_points("my_test", q, 2).await;
    // println!("Search result points are {:?}", r);

    // * compute embeddings for a query
    {
        println!("[+] Computing embeddings for a query ...");
        let query_text = "What is the capital of France?";
        let embedding_request = EmbeddingRequest {
            model: "dummy-embedding-model".to_string(),
            input: vec![query_text.to_string()],
            encoding_format: None,
            user: None,
        };
        let request_body = serde_json::to_value(&embedding_request).unwrap();

        let query_embedding = match client
            .post("http://localhost:8080/v1/embeddings")
            .header("accept", "application/json")
            .header("Content-Type", "application/json")
            .json(&request_body)
            .send()
            .await
        {
            Ok(response) => {
                let embedding_reponse: EmbeddingsResponse = response.json().await.unwrap();
                // println!(
                //     "Number of embedding objects: {}",
                //     embedding_reponse.data.len()
                // );

                embedding_reponse.data[0].clone()
            }
            Err(err) => {
                println!(
                    "Failed to compute embeddings for the user query. {}",
                    err.to_string()
                );
                return Err(err.to_string());
            }
        };

        // * search for similar points

        println!("[+] Searching for similar points ...");
        let query_vector = query_embedding
            .embedding
            .iter()
            .map(|x| *x as f32)
            .collect();

        let search_result = qdrant_client
            .search_points(collection_name, query_vector, 2)
            .await;

        for (i, point) in search_result.iter().enumerate() {
            println!("  *** Point {}: score: {}", i, point.score);

            if let Some(payload) = &point.payload {
                if let Some(source) = payload.get("source") {
                    println!("    Source: {}", source);
                }
            }
        }
    }

    Ok(())
}
