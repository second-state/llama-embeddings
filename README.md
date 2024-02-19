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
  wasmedge --dir .:. llama-embeddings.wasm --file bitcoin.txt
  ```

  If the command runs successfully, you will see the following output:

  ```console
  [+] Loading the text file ...
  [+] Chunking the text ...
  [+] Creating embeddings for the chunks ...
      * Number of embedding objects: 6
  [+] Creating a collection ...
      * Collection name: my_test
      * Dimension: 4096
  [+] Creating points to save embeddings ...
  [+] Upserting points ...


  =========== Tiny RAG Demo ===========

  [You] What is bitcoin?


  [+] Creating embeddings for the chunks ...
      * Number of embedding objects: 1
  [+] Searching for similar points ...
      * Point 0: score: 0.4680615
        Source: "uncertainties can be avoided in person by using physical currency, but no mechanism\nexists to make payments over a communications channel without a trusted party."
      * Point 1: score: 0.4614826
        Source: "basis, and nodes can leave and rejoin the network at will, accepting the longest\nproof-of-work chain as proof of what happened while they were gone. Bitcoin is a\npeer-to-peer electronic cash system."
      * Point 2: score: 0.20994519
        Source: "hash-based proof-of-work, forming a record that cannot be changed without redoing\nthe proof-of-work. The longest chain not only serves as proof of the sequence of\nevents witnessed, but proof that it came from the largest pool of CPU power. As\nlong as a majority of CPU power is controlled by nodes that are not cooperating to\nattack the network, they'll generate the longest chain and outpace attackers. The\nnetwork itself requires minimal structure. Messages are broadcast on a best effort"



  [Bot]: Based on the provided context, Bitcoin can be described as:

  * A peer-to-peer electronic cash system that allows for payments to be made without the need for a trusted party.
  * Using hash-based proof-of-work to form a record of transactions that cannot be changed without redoing the proof-of-work.
  * A decentralized network where nodes can leave and rejoin at will, accepting the longest proof-of-work chain as proof of what happened while they were gone.
  * Requiring minimal structure, with messages broadcast on a best-effort basis.

  In summary, Bitcoin is a decentralized digital currency that uses cryptography to secure and verify transactions without the need for a central authority.
  ```
