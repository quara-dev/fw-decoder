# Rust WebAssembly Log Decoder

This project is a one-page website built with Rust and WebAssembly. It features:
- Pulldown menu to choose version
- File input button to upload a file
- Text field to display information
- Pulldown menu to select log level, filtering displayed information

## Getting Started

1. Install Rust and wasm-pack
2. Build the project with `wasm-pack build --target web`
3. Serve the `index.html` file with a static file server

## Features
- Rust WebAssembly frontend
- Interactive UI for log decoding

## Development
- All code is in the `src` directory
- WebAssembly output is in the `pkg` directory after build

## License
MIT
