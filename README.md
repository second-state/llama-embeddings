# llama-embeddings

## Usage

Note that it is required to start `llama-api-server` before going through the following steps.

- Build

    ```bash
    cargo build --target wasm32-wasi --release
    ```

    Then, copy the generated `target/wasm32-wasi/release/llama_embeddings.wasm` to `llama-embeddings.wasm` in the root of this repository.

- Run

    ```bash
    wasmedge --dir .:. llama-embeddings.wasm --file <target-plain-text-file.txt>
    ```
