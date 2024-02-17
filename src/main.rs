use clap::{crate_version, Arg, ArgAction, Command};
use endpoints::embeddings::{EmbeddingRequest, EmbeddingsResponse};
use qdrant::*;
use std::fs::File;
use std::io::prelude::*;
use std::path::Path;

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
    let mut contents = String::new();
    file.read_to_string(&mut contents)
        .expect("failed to read file");

    println!("File contents: {}", contents);

    // * create embeddings

    let embedding_request = EmbeddingRequest {
        model: "fake-model-id".to_string(),
        input: vec![contents],
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
            // println!("Response: {:?}", embedding_reponse);
            println!(
                "Number of embedding objects: {}",
                embedding_reponse.data.len()
            );

            embedding_reponse.data
        }
        Err(err) => {
            println!("Error: {}", err);
            return Err(err.to_string());
        }
    };

    // todo: write the embeddings to a connected qdrant db

    {
        let collection_name = "my_test";

        let client = qdrant::Qdrant::new();
        let dim = 2048u32;

        // Create a collection with 10-dimensional vectors
        match client.create_collection(collection_name, dim).await {
            Ok(_) => {
                println!("Collection created");
            }
            Err(err) => {
                println!("Error: {}", err);
                return Err(err.to_string());
            }
        }

        let mut points = Vec::<Point>::new();

        for embedding in embeddings.iter() {
            let vector = embedding.embedding.iter().map(|x| *x as f32).collect();
            let p = Point {
                id: PointId::Num(embedding.index),
                vector,
                payload: None,
            };

            points.push(p);
        }

        // points.push(Point {
        //     id: PointId::Num(1),
        //     vector: vec![0.05, 0.61, 0.76, 0.74],
        //     payload: json!({"city": "Berlin"}).as_object().map(|m| m.to_owned()),
        // });
        // points.push(Point {
        //     id: PointId::Num(2),
        //     vector: vec![0.19, 0.81, 0.75, 0.11],
        //     payload: json!({"city": "London"}).as_object().map(|m| m.to_owned()),
        // });
        // points.push(Point {
        //     id: PointId::Num(3),
        //     vector: vec![0.36, 0.55, 0.47, 0.94],
        //     payload: json!({"city": "Moscow"}).as_object().map(|m| m.to_owned()),
        // });
        // points.push(Point {
        //     id: PointId::Num(4),
        //     vector: vec![0.18, 0.01, 0.85, 0.80],
        //     payload: json!({"city": "New York"})
        //         .as_object()
        //         .map(|m| m.to_owned()),
        // });
        // points.push(Point {
        //     id: PointId::Num(5),
        //     vector: vec![0.24, 0.18, 0.22, 0.44],
        //     payload: json!({"city": "Beijing"}).as_object().map(|m| m.to_owned()),
        // });
        // points.push(Point {
        //     id: PointId::Num(6),
        //     vector: vec![0.35, 0.08, 0.11, 0.44],
        //     payload: json!({"city": "Mumbai"}).as_object().map(|m| m.to_owned()),
        // });

        match client.upsert_points(collection_name, points).await {
            Ok(_) => {
                println!("Points upserted");
            }
            Err(err) => {
                println!("Error: {}", err);
                return Err(err.to_string());
            }
        }

        println!(
            "The collection size is {}",
            client.collection_info(collection_name).await
        );

        let p = client.get_point("my_test", 0).await;
        println!("The second point is {:?}", p);

        // let ps = client.get_points("my_test", vec![1, 2, 3, 4, 5, 6]).await;
        // println!("The 1-6 points are {:?}", ps);

        // let q = vec![0.2, 0.1, 0.9, 0.7];
        // let r = client.search_points("my_test", q, 2).await;
        // println!("Search result points are {:?}", r);

        match client.delete_points("my_test", vec![0]).await {
            Ok(_) => {
                println!("Point deleted");
            }
            Err(err) => {
                println!("Error: {}", err);
                return Err(err.to_string());
            }
        }

        println!(
            "The collection size is {}",
            client.collection_info("my_test").await
        );

        // let q = vec![0.2, 0.1, 0.9, 0.7];
        // let r = client.search_points("my_test", q, 2).await;
        // println!("Search result points are {:?}", r);
    }

    Ok(())
}
