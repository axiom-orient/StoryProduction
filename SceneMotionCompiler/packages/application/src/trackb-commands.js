import { canonicalHash, sha256Prefixed } from '../../core/src/canonical.js';
import { importFromNarrativeCourierReleaseBundle } from '../../adapters/src/narrative-courier-release.js';
import { composeSceneHtml } from '../../composer/src/scene-html.js';
import { parseTrackBSemanticManifestHtml } from '../../contracts/src/trackb-contract.js';
import { ApplicationCommandError } from './command-error.js';
import { measureTrackBEvidence } from './trackb-measure-runtime.js';
import { loadVerifiedSceneAssetCompilerHandoff } from './scene-asset-compiler-handoff.js';
import { publishSceneHtml } from './scene-html-output.js';

export const TRACK_B_HANDLER_IMPLEMENTATIONS = Object.freeze({
  async measureEvidence(config, input) {
    return measureTrackBEvidence(
      { projectRoot: config.projectRoot, input },
      config.trackBRuntimeDependencies
    );
  },

  async projectNarrativeBundle(config, input) {
    assertAllowedKeys(input, [
      'releaseBundlePath',
      'continuityBiblePath',
      'directorPlanPath',
      'directorApprovalPath'
    ]);
    return importFromNarrativeCourierReleaseBundle({
      releaseBundlePath: input.releaseBundlePath,
      continuityBiblePath: input.continuityBiblePath,
      directorPlanPath: input.directorPlanPath,
      directorApprovalPath: input.directorApprovalPath,
      projectRoot: config.projectRoot
    });
  },

  async compileHtml(config, input) {
    assertAllowedKeys(input, ['sceneMotionAssetHandoffPath', 'outputPath', 'width', 'height']);
    if (typeof input.sceneMotionAssetHandoffPath !== 'string' || input.sceneMotionAssetHandoffPath.trim().length === 0) {
      throw new ApplicationCommandError(
        'scene-motion-asset-handoff-required',
        'sceneMotionAssetHandoffPath is required for production Track B HTML output'
      );
    }

    const handoff = await loadVerifiedSceneAssetCompilerHandoff({
      projectRoot: config.projectRoot,
      handoffPath: input.sceneMotionAssetHandoffPath
    });
    const { sceneGraph } = handoff;
    const html = composeSceneHtml({
      sceneGraph,
      assets: handoff.assets,
      options: { width: input.width ?? 1280, height: input.height ?? 720 }
    });
    const htmlHash = sha256Prefixed(html);
    const embedding = parseTrackBSemanticManifestHtml(html);
    const projectionReceipt = handoff.projection.projectionReceipt;
    const narrativeId = projectionReceipt.narrativeId;
    const sourceSemanticManifestHash = sceneGraph.semanticManifest.sourceSemanticManifestHash;
    const semanticManifestHash = sceneGraph.semanticManifest.semanticManifestHash;
    const provenance = {
      schemaVersion: 2,
      narrativeId,
      projectionReceiptHash: handoff.projection.projectionReceiptHash,
      sceneGraphHash: canonicalHash(sceneGraph),
      sourceSemanticManifestHash,
      semanticManifestHash,
      htmlHash,
      embedding: {
        anchor: embedding.anchor,
        encoding: embedding.encoding,
        payloadHash: embedding.payloadHash
      },
      handoffHash: handoff.handoffHash,
      assetManifestHash: handoff.provenance.assetManifestHash
    };
    provenance.provenanceHash = canonicalHash(provenance);

    const { outputPath, provenancePath } = await publishSceneHtml({
      projectRoot: config.projectRoot,
      outputPath: input.outputPath ?? 'scene-output.html',
      html,
      provenance,
      protectedPaths: handoff.inputPaths
    });
    return {
      sceneGraphId: sceneGraph.sceneGraphId,
      sceneGraphHash: provenance.sceneGraphHash,
      narrativeId,
      projectionReceiptPath: handoff.projection.projectionReceiptPath,
      projectionReceiptHash: handoff.projection.projectionReceiptHash,
      shotCount: sceneGraph.shots.length,
      characterCount: sceneGraph.characters.length,
      environmentCount: sceneGraph.environments.length,
      outputBytes: Buffer.byteLength(html, 'utf8'),
      outputPath,
      htmlHash,
      handoffHash: provenance.handoffHash,
      assetManifestHash: handoff.provenance.assetManifestHash,
      provenancePath,
      provenance
    };
  }
});

function assertAllowedKeys(input, allowed) {
  if (!input || typeof input !== 'object' || Array.isArray(input)) {
    throw new ApplicationCommandError('command-input-invalid', 'Command input must be an object');
  }
  for (const key of Object.keys(input)) {
    if (!allowed.includes(key)) {
      throw new ApplicationCommandError('command-input-invalid', `unsupported input key: ${key}`);
    }
  }
}
