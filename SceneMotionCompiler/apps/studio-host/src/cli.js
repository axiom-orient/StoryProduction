import { createStudioServer } from './server.js';
import { resolve } from 'node:path';

const port = Number(process.env.STUDIO_PORT ?? 3456);
const args = process.argv.slice(2);
const positional = [];
let operatorId;
for (let index = 0; index < args.length; index += 1) {
  const arg = args[index];
  if (arg === '--operator-id') {
    operatorId = args[index + 1];
    index += 1;
  } else if (arg.startsWith('--operator-id=')) {
    operatorId = arg.slice('--operator-id='.length);
  } else if (arg.startsWith('--')) {
    throw new Error(`Unknown Studio option: ${arg}`);
  } else {
    positional.push(arg);
  }
}
const localOperatorId = operatorId ?? process.env.SMC_LOCAL_OPERATOR_ID ?? process.env.STUDIO_OPERATOR_ID;
const projectRoot = resolve(positional[0] ?? process.cwd());
const repoRoot = resolve(positional[1] ?? projectRoot);

const server = createStudioServer({ port, projectRoot, repoRoot, localOperatorId });
server.listen(port, '127.0.0.1', () => {
  const actualPort = server.address()?.port ?? port;
  console.log(`SceneMotionCompiler Studio → http://127.0.0.1:${actualPort} (project: ${projectRoot})`);
});
