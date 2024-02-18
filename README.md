# llama-embeddings

## Setup

- Start `llama-api-server`

  ```bash
  wasmedge --dir .:. --nn-preload default:GGML:AUTO:Llama-2-7b-chat-hf-Q5_K_M.gguf llama-api-server.wasm -p llama-2-chat
  ```

- Start Qdrant docker container

  ```console
  # Pull the Qdrant docker image
  docker pull qdrant/qdrant

  # Create a directory to store Qdrant data
  mkdir qdrant_storage

  # Run Qdrant service
  docker run -p 6333:6333 -p 6334:6334 -v $(pwd)/qdrant_storage:/qdrant/storage:z qdrant/qdrant
  ```

## Usage

- Build

  ```console
  # clone the repository
  git clone https://github.com/second-state/llama-embeddings.git
  cd llama-embeddings

  # build the wasm file
  cargo build --target wasm32-wasi --release
  ```

  Then, copy the generated `target/wasm32-wasi/release/llama_embeddings.wasm` to `llama-embeddings.wasm` in the root of this repository.

- Run

  ```bash
  wasmedge --dir .:. llama-embeddings.wasm --file paris.txt
  ```

  If the command runs successfully, you will see the following output:

  ```console
  [+] Reading text ...
  [+] Chunking the text ...
  [+] Creating embeddings for the chunks ...
      * Number of embedding objects: 6
  [+] Creating points to save embeddings ...
  [+] Creating a collection ...
      * Collection name: my_test
      * Dimension: 4096
  [+] Upserting points ...
  [+] Computing embeddings for a query ...
  [+] Searching for similar points ...
      * Point 0: score: 0.39630103
        Source: "Paris occupies a central position in the rich agricultural region known as the Paris Basin, and it constitutes one of eight départements of the Île-de-France administrative region. It is by far the country’s most important centre of commerce and culture. Area city, 41 square miles (105 square km); metropolitan area, 890 square miles (2,300 square km)."
      * Point 1: score: 0.35577077
        Source: "Paris, city and capital of France, situated in the north-central part of the country. People were living on the site of the present-day city, located along the Seine River some 233 miles (375 km) upstream from the river’s mouth on the English Channel (La Manche), by about 7600 BCE. The modern city has spread from the island (the Île de la Cité) and far beyond both banks of the Seine."
  ```
