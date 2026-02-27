function TextEncoder() {
}
TextEncoder.prototype.encode = function (string) {
    var octets = [];
    var length = string.length;
    var i = 0;
    while (i < length) {
        var codePoint = string.codePointAt(i);
        var c = 0;
        var bits = 0;
        if (codePoint <= 0x0000007F) {
            c = 0;
            bits = 0x00;
        } else if (codePoint <= 0x000007FF) {
            c = 6;
            bits = 0xC0;
        } else if (codePoint <= 0x0000FFFF) {
            c = 12;
            bits = 0xE0;
        } else if (codePoint <= 0x001FFFFF) {
            c = 18;
            bits = 0xF0;
        }
        octets.push(bits | (codePoint >> c));
        c -= 6;
        while (c >= 0) {
            octets.push(0x80 | ((codePoint >> c) & 0x3F));
            c -= 6;
        }
        i += codePoint >= 0x10000 ? 2 : 1;
    }
    return octets;
};
const REPLACEMENT = '\ufffd'
const BYTES = {
    0: 0,
    1: 0,
    2: 0,
    3: 0,
    4: 0,
    5: 0,
    6: 0,
    7: 0,
    8: 0,
    9: 0,
    10: 0,
    11: 0,
    12: 0,
    13: 0,
    14: 0,
    15: 0,
    16: -1,
    17: -1,
    18: -1,
    19: -1,
    20: -1,
    21: -1,
    22: -1,
    23: -1,
    24: 1,
    25: 1,
    26: 1,
    27: 1,
    28: 2,
    29: 2,
    30: 3,
    31: -2,
}

const ERR_MSG = '[ERR_ENCODING_INVALID_ENCODED_DATA]: ' +
    'The encoded data was not valid for encoding utf-8'

function utf8Decode(buf, fatal, state) {
    if (!state) {
        state = { cur: 0, left: 0 }
    }
    let res = ''
    for (const b of buf) {
        const bytes = BYTES[(b & 0xf8) >> 3]
        switch (bytes) {
            case -2:
                // Top 5 bits all set
                state.cur = 0
                state.left = 0
                if (fatal) {
                    const err = new TypeError(ERR_MSG)
                    err.code = 'ERR_ENCODING_INVALID_ENCODED_DATA'
                    err.errno = 12
                    throw err
                } else {
                    res += REPLACEMENT
                }
                break
            case -1:
                state.left--
                if (state.left < 0) {
                    // Too many continuation bytes
                    state.cur = 0
                    state.left = 0
                    if (fatal) {
                        const err = new TypeError(ERR_MSG)
                        err.code = 'ERR_ENCODING_INVALID_ENCODED_DATA'
                        err.errno = 12
                        throw err
                    } else {
                        res += REPLACEMENT
                    }
                } else {
                    state.cur = (state.cur << 6) | (b & 0x3f)
                    if (state.left === 0) {
                        res += String.fromCodePoint(state.cur)
                        state.cur = 0
                    }
                }
                break
            case 0: // One ASCII7 byte
                if ((state.cur !== 0) || (state.left !== 0)) {
                    // Not enough continuation bytes
                    state.cur = 0
                    state.left = 0
                    if (fatal) {
                        const err = new TypeError(ERR_MSG)
                        err.code = 'ERR_ENCODING_INVALID_ENCODED_DATA'
                        err.errno = 12
                        throw err
                    } else {
                        res += REPLACEMENT
                    }
                }
                res += String.fromCharCode(b)
                break
            default:
                if ((state.cur !== 0) || (state.left !== 0)) {
                    // Not enough continuation bytes
                    state.cur = 0
                    state.left = 0
                    if (fatal) {
                        const err = new TypeError(ERR_MSG)
                        err.code = 'ERR_ENCODING_INVALID_ENCODED_DATA'
                        err.errno = 12
                        throw err
                    } else {
                        res += REPLACEMENT
                    }
                }
                state.left = bytes
                state.cur = b & (0xff >> (bytes + 2))
                break
        }
    }
    return [res, state]
}

class TextDecoderPolyfill {
    constructor(utfLabel, options) {
        this.utfLabel = (utfLabel || 'utf-8').toLowerCase()
        if ((this.utfLabel !== 'utf-8') && (this.utfLabel !== 'utf8')) {
            const err = new RangeError('The "' + utfLabel + '" encoding is not supported')
            err.code = 'ERR_ENCODING_NOT_SUPPORTED'
            throw err
        }
        options = options || {}
        this.fatal = Boolean(options.fatal)
        this.ignoreBOM = Boolean(options.ignoreBOM)
        this.state = null
    }

    decode(input, options) {
        if (!(input instanceof Uint8Array)) {
            if (input instanceof ArrayBuffer) {
                input = new Uint8Array(input)
            } else if (ArrayBuffer.isView(input)) {
                input = new Uint8Array(input.buffer, input.byteOffset, input.byteLength)
            } else {
                const typ = typeof input
                const err = new TypeError('The "input" argument must be an instance of ArrayBuffer or ArrayBufferView. Received type ' + typ)
                err.code = 'ERR_INVALID_ARG_TYPE'
                throw err
            }
        }
        const str_state = utf8Decode(input, this.fatal, this.state)
        let str = str_state[0]
        const state = str_state[1]
        if (options && options.stream) {
            this.state = state
        } else {
            this.state = null
            if (state.left !== 0) {
                // Truncated
                if (this.fatal) {
                    const err = new TypeError(ERR_MSG)
                    err.code = 'ERR_ENCODING_INVALID_ENCODED_DATA'
                    err.errno = 11
                    throw err
                } else {
                    str += REPLACEMENT
                }
            }
        }

        if (!this.ignoreBOM) {
            if (str.codePointAt(0) === 0xFEFF) {
                return str.slice(1)
            }
        }
        return str
    }
}
globalThis.TextDecoder = TextDecoderPolyfill
globalThis.crypto = {
    getRandomValues(array) {
        for (let i = 0; i < array.length; i++) {
            array[i] = Math.floor(Math.random() * 256);
        }

        return array;
    }
};
const scriptStartTime = Date.now();
globalThis.performance = {
    now() { return Date.now() - scriptStartTime; }
};
var Module = {
    preRun: [],
    postRun: [],
    print: function (text) {
        console.log(text);
    },
    printErr: function (text) {
        console.error(text);
    }
};
