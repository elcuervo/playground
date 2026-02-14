import { DurableObject } from "cloudflare:workers";
import mod from "./mrbe_playground_api_core.wasm";

// Type declaration for WebAssembly module import
const wasmModule: WebAssembly.Module = mod as WebAssembly.Module;

/**
 * Welcome to Cloudflare Workers! This is your first Durable Objects application.
 *
 * - Run `npm run dev` in your terminal to start a development server
 * - Open a browser tab at http://localhost:8787/ to see your Durable Object in action
 * - Run `npm run deploy` to publish your application
 *
 * Bind resources to your worker in `wrangler.jsonc`. After adding bindings, a type definition for the
 * `Env` object can be regenerated with `npm run cf-typegen`.
 *
 * Learn more at https://developers.cloudflare.com/durable-objects
 */

/** A Durable Object's behavior is defined in an exported Javascript class */
export class MrbePlaygroundObject extends DurableObject<Env> {
	/**
	 * The constructor is invoked once upon creation of the Durable Object, i.e. the first call to
	 * 	`DurableObjectStub::get` for a given identifier (no-op constructors can be omitted)
	 *
	 * @param ctx - The interface for interacting with Durable Object state
	 * @param env - The interface to reference bindings declared in wrangler.jsonc
	 */
	constructor(ctx: DurableObjectState, env: Env) {
		super(ctx, env);
	}

	/**
	 * Get a value from Durable Object storage
	 *
	 * @param key - The key to retrieve
	 * @returns The value associated with the key, or null if not found
	 */
	async get(key: string): Promise<string | null> {
		const value = await this.ctx.storage.get<string>(key);
		return value ?? null;
	}

	/**
	 * Set a value in Durable Object storage
	 *
	 * @param key - The key to store
	 * @param value - The value to store
	 */
	async set(key: string, value: string): Promise<void> {
		await this.ctx.storage.put(key, value);
	}
}

export default {
	/**
	 * This is the standard fetch handler for a Cloudflare Worker
	 *
	 * @param request - The request submitted to the Worker from the client
	 * @param env - The interface to reference bindings declared in wrangler.jsonc
	 * @param ctx - The execution context of the Worker
	 * @returns The response to be sent back to the client
	 */
	async fetch(request, env, ctx): Promise<Response> {
		const path = new URL(request.url).pathname;
		if (path === "/favicon.ico") {
			return new Response(null, { status: 404 });
		}
		const query = new URL(request.url).searchParams;

		// Create a stub to open a communication channel with the Durable Object
		// instance named "foo".
		//
		// Requests from all Workers to the Durable Object instance named "foo"
		// will go to a single remote Durable Object instance.
		const stub = env.MRBE_PLAYGROUND_DATA.getByName("foo");

		// Storage operations buffer for async processing
		const pendingOperations: Array<{ type: 'set', key: string, value: string } | { type: 'get', key: string, resolve: (value: string | null) => void }> = [];
		const cachedValues = new Map<string, string | null>();

		const key = query.get("key");
		if (request.method === "GET" && key) {
			// preflight get value and cache it to avoid round-trip during request processing
			const value = await stub.get(key);
			cachedValues.set(key, value);
			console.log(`[Preflight] GET ${key} => ${value}`);
		}

		// Create decoder/encoder once for reuse
		const decoder = new TextDecoder();
		const encoder = new TextEncoder();

		const importObject = {
			env: {
				debug_console_log: (ptr: number, size: number) => {
					const memory = exports.memory;
					const buffer = new Uint8Array(memory.buffer, ptr, size);
					console.log(`[debug]: ${decoder.decode(buffer)}`);
					return 0;
				},
				// Get a value from cache and write to WebAssembly memory
				// Returns the length of the value, or -1 if not found
				// Value is written to the buffer at resultPtr
				do_storage_get: (keyPtr: number, keySize: number, resultPtr: number, resultMaxSize: number): number => {
					const memory = exports.memory;
					const keyBuffer = new Uint8Array(memory.buffer, keyPtr, keySize);
					const key = decoder.decode(keyBuffer);

					// Check if we have a cached value
					if (cachedValues.has(key)) {
						const value = cachedValues.get(key);
						if (value === null) {
							return -1;
						}
						const valueBytes = encoder.encode(value);
						const resultBuffer = new Uint8Array(memory.buffer, resultPtr, resultMaxSize);
						const length = Math.min(valueBytes.length, resultMaxSize);
						resultBuffer.set(valueBytes.slice(0, length));
						return length;
					}

					// Value not in cache, return -1
					return -1;
				},
				// Set a value in Durable Object storage (buffered)
				do_storage_set: (keyPtr: number, keySize: number, valuePtr: number, valueSize: number): number => {
					const memory = exports.memory;
					const keyBuffer = new Uint8Array(memory.buffer, keyPtr, keySize);
					const key = decoder.decode(keyBuffer);
					const valueBuffer = new Uint8Array(memory.buffer, valuePtr, valueSize);
					const value = decoder.decode(valueBuffer);

					// Buffer the operation for async execution
					pendingOperations.push({ type: 'set', key, value });
					// Update cache
					cachedValues.set(key, value);
					return 0;
				},
			},
		};
		const instance = await WebAssembly.instantiate(wasmModule, importObject);
		const exports = instance.exports;

		// Process request
		const reqResult = exports.uzumibi_initialize_request(65536);
		const reqOffset = Number(reqResult & 0xFFFFFFFFn);
		if (reqOffset === 0) {
			const errOffset = Number((reqResult >> 32n) & 0xFFFFFFFFn);
			const buffer = new Uint8Array(exports.memory.buffer, errOffset);
			let errStr = "";
			for (let i = 0; buffer[i] !== 0; i++) {
				errStr += String.fromCharCode(buffer[i]);
			}
			throw new Error(`Failed to initialize request: ${errStr}`);
		}
		const requestBuffer = new Uint8Array(exports.memory.buffer, reqOffset, 65536);

		let pos = 0;
		const dataView = new DataView(exports.memory.buffer, reqOffset);

		const method = encoder.encode(request.method);
		requestBuffer.fill(0, pos, pos + 6);
		requestBuffer.set(method.slice(0, 6), pos);
		pos += 6;

		// Path size (u16 little-endian)
		const pathBytes = encoder.encode(path);
		dataView.setUint16(pos, pathBytes.length, true);
		pos += 2;

		// Path
		requestBuffer.set(pathBytes, pos);
		pos += pathBytes.length;

		// Query string size (u16 little-endian)
		const queryString = query.toString();
		const queryBytes = encoder.encode(queryString);
		dataView.setUint16(pos, queryBytes.length, true);
		pos += 2;

		// Query string
		requestBuffer.set(queryBytes, pos);
		pos += queryBytes.length;

		// Headers
		const headers: { key: string; value: string; }[] = [];
		request.headers.forEach((value: string, key: string) => {
			// 一般的なヘッダーのみ含める（必要に応じて調整）
			if (key.toLowerCase() !== 'cf-connecting-ip' &&
				key.toLowerCase() !== 'cf-ray' &&
				!key.toLowerCase().startsWith('x-')) {
				headers.push({ key, value });
			}
		});

		// Headers count (u16 little-endian)
		dataView.setUint16(pos, headers.length, true);
		pos += 2;

		// Each header
		for (const header of headers) {
			// Header key size (u16 little-endian)
			const keyBytes = encoder.encode(header.key);
			dataView.setUint16(pos, keyBytes.length, true);
			pos += 2;

			// Header key
			requestBuffer.set(keyBytes, pos);
			pos += keyBytes.length;

			// Header value size (u16 little-endian)
			const valueBytes = encoder.encode(header.value);
			dataView.setUint16(pos, valueBytes.length, true);
			pos += 2;

			// Header value
			requestBuffer.set(valueBytes, pos);
			pos += valueBytes.length;
		}

		// Request body size (u32 little-endian)
		const bodyBytes = request.body ? new Uint8Array(await request.arrayBuffer()) : new Uint8Array(0);
		dataView.setUint32(pos, bodyBytes.length, true);
		pos += 4;

		// Request body
		requestBuffer.set(bodyBytes, pos);
		pos += bodyBytes.length;

		if (pos > 65536) {
			throw new Error("Request data exceeds allocated buffer size");
		}

		const resResult = exports.uzumibi_start_request();
		const resOffset = Number(resResult & 0xFFFFFFFFn);
		if (resOffset === 0) {
			const errOffset = Number((resResult >> 32n) & 0xFFFFFFFFn);
			const buffer = new Uint8Array(exports.memory.buffer, errOffset);
			let errStr = "";
			for (let i = 0; buffer[i] !== 0; i++) {
				errStr += String.fromCharCode(buffer[i]);
			}
			throw new Error(`Failed to start request: ${errStr}`);
		}

		// Unpack response
		const resDataView = new DataView(exports.memory.buffer, resOffset);


		let resPos = 0;

		// Status code (u16 little-endian)
		const statusCode = resDataView.getUint16(resPos, true);
		resPos += 2;

		// Headers count (u16 little-endian)
		const headersCount = resDataView.getUint16(resPos, true);
		resPos += 2;

		// Parse headers
		const responseHeaders = new Headers();
		for (let i = 0; i < headersCount; i++) {
			// Header key size (u16 little-endian)
			const keySize = resDataView.getUint16(resPos, true);
			resPos += 2;

			// Header key
			const keyBytes = new Uint8Array(exports.memory.buffer, resOffset + resPos, keySize);
			const key = decoder.decode(keyBytes);
			resPos += keySize;

			// Header value size (u16 little-endian)
			const valueSize = resDataView.getUint16(resPos, true);
			resPos += 2;

			// Header value
			const valueBytes = new Uint8Array(exports.memory.buffer, resOffset + resPos, valueSize);
			const value = decoder.decode(valueBytes);
			resPos += valueSize;

			console.log(`[Response Header] ${key}: ${value}`);
			responseHeaders.set(key, value);
		}

		// Body size (u32 little-endian)
		const bodySize = resDataView.getUint32(resPos, true);
		resPos += 4;

		// Body
		const bodyBuffer = new Uint8Array(exports.memory.buffer, resOffset + resPos, bodySize);
		const responseText = decoder.decode(bodyBuffer);

		// Execute all pending storage operations
		for (const op of pendingOperations) {
			if (op.type === 'set') {
				await stub.set(op.key, op.value);
			} else if (op.type === 'get') {
				const value = await stub.get(op.key);
				op.resolve(value);
			}
		}

		return new Response(responseText, { status: statusCode, headers: responseHeaders });
	},
} satisfies ExportedHandler<Env>;
