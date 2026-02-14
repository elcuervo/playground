import { DurableObject } from "cloudflare:workers";
import mod from "./mrbe_playground_api_core.wasm";

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
export class MrbePlaygroundApi extends DurableObject<Env> {
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
	 * The Durable Object exposes an RPC method sayHello which will be invoked when a Durable
	 *  Object instance receives a request from a Worker via the same method invocation on the stub
	 *
	 * @param name - The name provided to a Durable Object instance from a Worker
	 * @returns The greeting to be sent back to the Worker
	 */
	async sayHello(name: string): Promise<string> {
		return `Hello, ${name}!`;
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
		const importObject = {
			env: {
				debug_console_log: (ptr, size) => {
					const memory = exports.memory;
					let str = "";
					const buffer = new Uint8Array(memory.buffer);
					for (let i = ptr; i < ptr + size; i++) {
						str += String.fromCharCode(buffer[i]);
					}
					console.log(`[debug]: ${str}`);
					return 0;
				},
			},
		};
		const instance = await WebAssembly.instantiate(mod, importObject);
		const exports = instance.exports;

		// Create a stub to open a communication channel with the Durable Object
		// instance named "foo".
		//
		// Requests from all Workers to the Durable Object instance named "foo"
		// will go to a single remote Durable Object instance.
		const stub = env.MRBE_PLAYGROUND_DATA.getByName("foo");

		// Call the `sayHello()` RPC method on the stub to invoke the method on
		// the remote Durable Object instance.
		const greeting = await stub.sayHello("world");

		const reqResult = exports.uzumibi_initialize_request(65536);
		const reqOffset = Number(reqResult & 0xFFFFFFFFn);
		if (reqOffset === 0) {
			const errOffset = Number((reqResult >> 32n) & 0xFFFFFFFFn);
			const decoder = new TextDecoder();
			let errStr = "";
			const buffer = new Uint8Array(exports.memory.buffer, errOffset);
			for (let i = 0; buffer[i] !== 0; i++) {
				errStr += String.fromCharCode(buffer[i]);
			}
			throw new Error(`Failed to initialize request: ${errStr}`);
		}
		const requestBuffer = new Uint8Array(exports.memory.buffer, reqOffset, 65536);
		const path = new URL(request.url).pathname;
		if (path === "/favicon.ico") {
			return new Response(null, { status: 404 });
		}

		const query = new URL(request.url).searchParams;

		let pos = 0;
		const encoder = new TextEncoder();
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
		const headers = [];
		request.headers.forEach((value, key) => {
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
			const decoder = new TextDecoder();
			let errStr = "";
			const buffer = new Uint8Array(exports.memory.buffer, errOffset);
			for (let i = 0; buffer[i] !== 0; i++) {
				errStr += String.fromCharCode(buffer[i]);
			}
			throw new Error(`Failed to start request: ${errStr}`);
		}

		// Unpack response
		const decoder = new TextDecoder();
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

		return new Response(responseText, { status: statusCode, headers: responseHeaders });
	},
} satisfies ExportedHandler<Env>;
