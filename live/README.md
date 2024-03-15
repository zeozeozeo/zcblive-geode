# Building


- for geode (make sure `crate-type` in Cargo.toml is set to `staticlib`):
    ```
    cargo build --release --features geode
    ```
    
    then, to build the Geode wrapper ([docs](https://docs.geode-sdk.org/getting-started/create-mod/#build)):

    ```
    $ cd ..
    $ cmake --build build --config Release
    ```

- as a DLL (make sure `crate-type` in Cargo.toml is set to `cdylib`):
    ```
    cargo build --release
    ```

# Details

- `--features geode` disables compiling a lot of DLL-specific code
