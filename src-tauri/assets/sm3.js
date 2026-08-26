/* SM3 国密哈希（GB/T 32905-2016）—— 移植自 sm-crypto（MIT License，github.com/JuneAndGreen/sm-crypto）。
   纯 JS 实现、无外部依赖，可在 boa_engine / 浏览器运行。
   全局 API：
     sm3(str)            → 64 位小写 hex（与 gm-crypto 的 sm3(data) 默认 hex 输出一致）
     SM3.hex(str)        → 同上
     SM3.hmac(str, key)  → SM3-HMAC（key 为 hex 字符串）
*/
(function (global) {
  'use strict';

  // ---- sm3 核心（来自 sm-crypto src/sm2/sm3.js）----
  var W = new Uint32Array(68);
  var M = new Uint32Array(64);

  function rotl(x, n) {
    var s = n & 31;
    return (x << s) | (x >>> (32 - s));
  }
  function xor(x, y) {
    var result = [];
    for (var i = x.length - 1; i >= 0; i--) result[i] = (x[i] ^ y[i]) & 0xff;
    return result;
  }
  function P0(X) {
    return (X ^ rotl(X, 9)) ^ rotl(X, 17);
  }
  function P1(X) {
    return (X ^ rotl(X, 15)) ^ rotl(X, 23);
  }

  function sm3Core(array) {
    var len = array.length * 8;
    var k = len % 512;
    k = k >= 448 ? 512 - (k % 448) - 1 : 448 - k - 1;
    var kArr = new Array((k - 7) / 8);
    var lenArr = new Array(8);
    for (var i = 0; i < kArr.length; i++) kArr[i] = 0;
    for (var i = 0; i < lenArr.length; i++) lenArr[i] = 0;
    len = len.toString(2);
    for (var i = 7; i >= 0; i--) {
      if (len.length > 8) {
        var start = len.length - 8;
        lenArr[i] = parseInt(len.slice(start), 2);
        len = len.slice(0, start);
      } else if (len.length > 0) {
        lenArr[i] = parseInt(len, 2);
        len = '';
      }
    }
    var m = new Uint8Array(array.length + 1 + kArr.length + 8);
    m.set(array, 0);
    m.set([0x80], array.length);
    m.set(kArr, array.length + 1);
    m.set(lenArr, array.length + 1 + kArr.length);
    var dataView = new DataView(m.buffer, 0);
    var n = m.length / 64;
    var V = new Uint32Array([
      0x7380166f, 0x4914b2b9, 0x172442d7, 0xda8a0600,
      0xa96f30bc, 0x163138aa, 0xe38dee4d, 0xb0fb0e4e,
    ]);
    for (var i = 0; i < n; i++) {
      W.fill(0);
      M.fill(0);
      var start = 16 * i;
      for (var j = 0; j < 16; j++) W[j] = dataView.getUint32((start + j) * 4, false);
      for (var j = 16; j < 68; j++) {
        W[j] = (P1((W[j - 16] ^ W[j - 9]) ^ rotl(W[j - 3], 15)) ^ rotl(W[j - 13], 7)) ^ W[j - 6];
      }
      for (var j = 0; j < 64; j++) M[j] = W[j] ^ W[j + 4];
      var T1 = 0x79cc4519;
      var T2 = 0x7a879d8a;
      var A = V[0], B = V[1], C = V[2], D = V[3];
      var E = V[4], F = V[5], G = V[6], H = V[7];
      var SS1, SS2, TT1, TT2, T;
      for (var j = 0; j < 64; j++) {
        T = j <= 15 ? T1 : T2;
        SS1 = rotl(rotl(A, 12) + E + rotl(T, j), 7);
        SS2 = SS1 ^ rotl(A, 12);
        TT1 = (j <= 15 ? ((A ^ B) ^ C) : (((A & B) | (A & C)) | (B & C))) + D + SS2 + M[j];
        TT2 = (j <= 15 ? ((E ^ F) ^ G) : ((E & F) | ((~E) & G))) + H + SS1 + W[j];
        D = C; C = rotl(B, 9); B = A; A = TT1;
        H = G; G = rotl(F, 19); F = E; E = P0(TT2);
      }
      V[0] ^= A; V[1] ^= B; V[2] ^= C; V[3] ^= D;
      V[4] ^= E; V[5] ^= F; V[6] ^= G; V[7] ^= H;
    }
    var result = [];
    for (var i = 0; i < V.length; i++) {
      var word = V[i];
      result.push((word & 0xff000000) >>> 24, (word & 0xff0000) >>> 16, (word & 0xff00) >>> 8, word & 0xff);
    }
    return result;
  }

  // ---- utf8 / hex 编解码（来自 sm-crypto src/sm3/index.js）----
  function leftPad(input, num) {
    if (input.length >= num) return input;
    return new Array(num - input.length + 1).join('0') + input;
  }
  function ArrayToHex(arr) {
    return arr
      .map(function (item) {
        item = item.toString(16);
        return item.length === 1 ? '0' + item : item;
      })
      .join('');
  }
  function hexToArray(hexStr) {
    var words = [];
    if (hexStr.length % 2 !== 0) hexStr = leftPad(hexStr, hexStr.length + 1);
    for (var i = 0; i < hexStr.length; i += 2) {
      words.push(parseInt(hexStr.slice(i, i + 2), 16));
    }
    return words;
  }
  function utf8ToArray(str) {
    var arr = [];
    for (var i = 0, len = str.length; i < len; i++) {
      var point = str.codePointAt(i);
      if (point <= 0x007f) {
        arr.push(point);
      } else if (point <= 0x07ff) {
        arr.push(0xc0 | (point >>> 6));
        arr.push(0x80 | (point & 0x3f));
      } else if (point <= 0xd7ff || (point >= 0xe000 && point <= 0xffff)) {
        arr.push(0xe0 | (point >>> 12));
        arr.push(0x80 | ((point >>> 6) & 0x3f));
        arr.push(0x80 | (point & 0x3f));
      } else if (point >= 0x010000 && point <= 0x10ffff) {
        i++;
        arr.push(0xf0 | ((point >>> 18) & 0x1c));
        arr.push(0x80 | ((point >>> 12) & 0x3f));
        arr.push(0x80 | ((point >>> 6) & 0x3f));
        arr.push(0x80 | (point & 0x3f));
      } else {
        throw new Error('input is not supported');
      }
    }
    return arr;
  }

  // ---- 对外 API ----
  function toBytes(input) {
    return typeof input === 'string' ? utf8ToArray(input) : Array.prototype.slice.call(input);
  }

  var SM3 = {
    hex: function (input) {
      return ArrayToHex(sm3Core(toBytes(input)));
    },
    hash: function (input) {
      return ArrayToHex(sm3Core(toBytes(input)));
    },
    hmac: function (input, keyHex) {
      var inputArr = toBytes(input);
      var key = hexToArray(String(keyHex));
      if (key.length > 64) key = sm3Core(key);
      while (key.length < 64) key.push(0);
      var iPad = new Uint8Array(64);
      var oPad = new Uint8Array(64);
      for (var i = 0; i < 64; i++) {
        iPad[i] = 0x36;
        oPad[i] = 0x5c;
      }
      var inner = sm3Core(xor(key, iPad).concat(inputArr));
      return ArrayToHex(sm3Core(xor(key, oPad).concat(inner)));
    },
  };

  global.SM3 = SM3;
  /* 便捷全局函数：sm3('abc') → 66c7f0f4...（与 gm-crypto 的 sm3(data) hex 输出一致） */
  global.sm3 = function (str) {
    return SM3.hex(str);
  };
})(typeof globalThis !== 'undefined' ? globalThis : this);
