import * as THREE from 'three';
import { createStrandMaterial, createRungMaterial, createParticleMaterial } from './materials.js';
import { createDNA } from './dna.js';
import { createParticles, updateParticles } from './particles.js';
import { setupInteraction, updateInteraction } from './interaction.js';
import { setupPostProcessing } from './postprocess.js';

const container = document.getElementById('dna-canvas-container');
if (!container) throw new Error('Missing #dna-canvas-container');

// --- Renderer (sized to container, not window) ---
const renderer = new THREE.WebGLRenderer({ antialias: true, alpha: true });
renderer.setPixelRatio(Math.min(window.devicePixelRatio, 2));
renderer.setClearColor(0x000000, 0);
renderer.toneMapping = THREE.ACESFilmicToneMapping;
container.appendChild(renderer.domElement);

// --- Scene ---
const scene = new THREE.Scene();

// --- Camera ---
const camera = new THREE.PerspectiveCamera(60, 1, 0.1, 100);
camera.position.set(0, 0, 12);
camera.lookAt(0, 0, 0);

// --- Lighting ---
scene.add(new THREE.AmbientLight(0xffffff, 0.15));
const lightCyan = new THREE.PointLight(0x00d4ff, 0.5, 50);
lightCyan.position.set(5, 5, 5);
scene.add(lightCyan);
const lightPurple = new THREE.PointLight(0xa855f7, 0.5, 50);
lightPurple.position.set(-5, -5, 5);
scene.add(lightPurple);

// --- DNA ---
const strandMaterials = {
  strandA: createStrandMaterial(0x00d4ff),
  strandB: createStrandMaterial(0xa855f7),
  rung: createRungMaterial(),
};
const { group: dnaGroup, config: dnaConfig } = createDNA(strandMaterials);
scene.add(dnaGroup);

// --- Particles ---
const particlesMesh = createParticles(dnaConfig, createParticleMaterial());
dnaGroup.add(particlesMesh);

// --- Interaction ---
setupInteraction(dnaGroup);

// --- Post-processing ---
const { composer } = setupPostProcessing(renderer, scene, camera);

// --- Resize to container ---
function resizeToContainer() {
  const rect = container.getBoundingClientRect();
  const w = rect.width;
  const h = rect.height;
  if (w === 0 || h === 0) return;
  camera.aspect = w / h;
  camera.updateProjectionMatrix();
  renderer.setSize(w, h);
  composer.setSize(w, h);
}
resizeToContainer();

let resizeTimeout;
window.addEventListener('resize', () => {
  clearTimeout(resizeTimeout);
  resizeTimeout = setTimeout(resizeToContainer, 100);
});

const resizeObserver = new ResizeObserver(() => resizeToContainer());
resizeObserver.observe(container);

// --- Animation Loop ---
let lastTime = 0;

function animate(time) {
  requestAnimationFrame(animate);
  const t = time * 0.001;
  const delta = lastTime ? t - lastTime : 0.016;
  lastTime = t;

  updateInteraction(t, delta);
  updateParticles(t, dnaConfig);
  composer.render();
}

animate(0);
