// JSON.parse validates grammar, but erases duplicate keys before contract
// validation. Inspect the validated token stream before returning its object.
export function parseStrictJson(bytes) {
  const text = typeof bytes === 'string' ? bytes : new TextDecoder('utf-8', { fatal: true }).decode(bytes);
  const value = JSON.parse(text);
  const stack = [];
  const tokens = /"(?:\\[\s\S]|[^"\\])*"|[{}\[\]:,]/g;
  for (const match of text.matchAll(tokens)) {
    const token = match[0];
    const top = stack.at(-1);
    if (token === '{') stack.push({ object: true, keys: new Set(), expectKey: true });
    else if (token === '[') stack.push({ object: false });
    else if (token === '}' || token === ']') stack.pop();
    else if (token === ',' && top?.object) top.expectKey = true;
    else if (token === ':' && top?.object) top.expectKey = false;
    else if (token.startsWith('"') && top?.object && top.expectKey) {
      const key = JSON.parse(token);
      if (top.keys.has(key)) throw new Error(`DUPLICATE_JSON_KEY:${key}`);
      top.keys.add(key);
    }
  }
  return value;
}
