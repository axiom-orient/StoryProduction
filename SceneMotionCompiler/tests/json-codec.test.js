import test from 'node:test';
import assert from 'node:assert/strict';
import { parseStrictJson } from '../packages/core/src/json-codec.js';

test('duplicate keys are rejected before material decisions; escaped names count as the same key', () => {
  for (const text of ['{"decision":"rejected","decision":"approved"}', '{"a":0,"\\u0061":1}', '{"x":[{"a":1,"a":2}]}'])
    assert.throws(() => parseStrictJson(text), /DUPLICATE_JSON_KEY/);
  assert.deepEqual(parseStrictJson('{"a":[{"a":"}:,\\\""},{"a":2}],"b":{"a":3}}'),
    { a: [{ a: '}:,"' }, { a: 2 }], b: { a: 3 } });
  assert.throws(() => parseStrictJson(Buffer.from([0xff])), /encoded data/);
});
