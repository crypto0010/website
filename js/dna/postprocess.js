import { EffectComposer } from 'three/addons/postprocessing/EffectComposer.js';
import { RenderPass } from 'three/addons/postprocessing/RenderPass.js';
import { UnrealBloomPass } from 'three/addons/postprocessing/UnrealBloomPass.js';
import { OutputPass } from 'three/addons/postprocessing/OutputPass.js';
import * as THREE from 'three';

/**
 * Sets up the EffectComposer with bloom.
 * @param {THREE.WebGLRenderer} renderer
 * @param {THREE.Scene} scene
 * @param {THREE.Camera} camera
 * @returns {EffectComposer}
 */
export function setupPostProcessing(renderer, scene, camera) {
  const size = renderer.getSize(new THREE.Vector2());

  const composer = new EffectComposer(renderer);
  composer.addPass(new RenderPass(scene, camera));

  const bloomPass = new UnrealBloomPass(
    new THREE.Vector2(size.x * 0.5, size.y * 0.5), // half resolution
    0.4,  // strength
    0.2,  // radius
    0.6   // threshold
  );
  composer.addPass(bloomPass);

  // OutputPass applies tone mapping + color space conversion
  composer.addPass(new OutputPass());

  return { composer, bloomPass };
}
