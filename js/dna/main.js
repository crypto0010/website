import * as THREE from 'three';
import GUI from 'lil-gui';
import { createStrandMaterial, createRungMaterial, createParticleMaterial } from './materials.js';
import { createDNA } from './dna.js';
import { createParticles, updateParticles, particleSettings } from './particles.js';
import { setupInteraction, updateInteraction, interactionSettings } from './interaction.js';
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
const ambientLight = new THREE.AmbientLight(0xffffff, 0.15);
scene.add(ambientLight);
const lightCyan = new THREE.PointLight(0x00d4ff, 0.5, 50);
lightCyan.position.set(5, 5, 5);
scene.add(lightCyan);
const lightPurple = new THREE.PointLight(0xa855f7, 0.5, 50);
lightPurple.position.set(-5, -5, 5);
scene.add(lightPurple);

// --- DNA (colors matched to portfolio theme) ---
const strandMaterials = {
  strandA: createStrandMaterial(0x00d4ff),
  strandB: createStrandMaterial(0xa855f7),
  rung: createRungMaterial(),
};
const { group: dnaGroup, config: dnaConfig } = createDNA(strandMaterials);
scene.add(dnaGroup);

// --- Particles ---
const particleMaterial = createParticleMaterial();
const particlesMesh = createParticles(dnaConfig, particleMaterial);
dnaGroup.add(particlesMesh);

// --- Interaction ---
setupInteraction(dnaGroup);

// --- Post-processing ---
const { composer, bloomPass } = setupPostProcessing(renderer, scene, camera);

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

// Also resize when the section becomes visible (intersection observer)
const resizeObserver = new ResizeObserver(() => resizeToContainer());
resizeObserver.observe(container);

// --- GUI Controls ---
const gui = new GUI({ title: 'DNA Controls' });
const guiContainer = document.getElementById('dna-gui-container');
guiContainer.appendChild(gui.domElement);

const bloomFolder = gui.addFolder('Bloom');
bloomFolder.add(bloomPass, 'strength', 0, 3, 0.05).name('Strength');
bloomFolder.add(bloomPass, 'radius', 0, 1, 0.05).name('Radius');
bloomFolder.add(bloomPass, 'threshold', 0, 1, 0.05).name('Threshold');

const strandFolder = gui.addFolder('Strands');
strandFolder.add(strandMaterials.strandA.uniforms.uEmissiveIntensity, 'value', 0, 3, 0.1)
  .name('Emissive')
  .onChange((v) => {
    strandMaterials.strandB.uniforms.uEmissiveIntensity.value = v;
  });

const lightFolder = gui.addFolder('Lighting');
lightFolder.add(ambientLight, 'intensity', 0, 1, 0.05).name('Ambient');
lightFolder.add(lightCyan, 'intensity', 0, 2, 0.1).name('Cyan Light');
lightFolder.add(lightPurple, 'intensity', 0, 2, 0.1).name('Purple Light');

const rungFolder = gui.addFolder('Rungs');
rungFolder.add(strandMaterials.rung, 'emissiveIntensity', 0, 2, 0.05).name('Emissive');

const particleFolder = gui.addFolder('Particles');
particleFolder.add(particleMaterial.uniforms.uSizeScale, 'value', 0.1, 3, 0.1).name('Size');
particleFolder.add(particleMaterial.uniforms.uOpacity, 'value', 0, 1, 0.05).name('Opacity');
particleFolder.add(particleSettings, 'scatterRadius', 0.1, 3, 0.1).name('Scatter Radius');
particleFolder.add(particleSettings, 'orbitSpeed', 0, 1, 0.05).name('Orbit Speed');
particleFolder.add(particleSettings, 'floatAmount', 0, 1, 0.05).name('Float Amount');
particleFolder.add(particleSettings, 'pulseAmount', 0, 1, 0.05).name('Pulse Amount');

const motionFolder = gui.addFolder('Motion');
motionFolder.add(interactionSettings, 'rotationSpeed', 0, 0.5, 0.01).name('Rotation Speed');
motionFolder.add(interactionSettings, 'floatAmplitude', 0, 1, 0.05).name('Float Amplitude');
motionFolder.add(interactionSettings, 'maxTilt', 0, 0.8, 0.02).name('Mouse Tilt');

// Start all folders closed
gui.controllersRecursive().forEach(c => {});
bloomFolder.close();
strandFolder.close();
lightFolder.close();
rungFolder.close();
particleFolder.close();
motionFolder.close();

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
