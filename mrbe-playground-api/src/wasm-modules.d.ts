// Global ambient declaration for .wasm module imports
// This file must NOT have any top-level export/import statements
declare module "*.wasm" {
	const module: WebAssembly.Module;
	export default module;
}
