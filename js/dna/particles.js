import * as THREE from 'three';

const PARTICLE_COUNT = 400;

export const particleSettings = {
  scatterRadius: 0.8,
  orbitSpeed: 0.3,
  floatAmount: 0.3,
  floatSpeed: 0.4,
  pulseAmount: 0.3,
  pulseSpeed: 0.5,
  sizeScale: 1.0,
  opacity: 1.0,
};

// Per-particle phase data (set once, read each frame)
let phases = null;
let geometry = null;

/**
 * Creates the orbiting particle system around the DNA helix.
 * @param {{ helixRadius: number, helixHeight: number, turns: number }} config
 * @param {THREE.ShaderMaterial} material
 * @returns {THREE.Points}
 */
export function createParticles(config, material) {
  const { helixRadius, helixHeight, turns } = config;
  const halfHeight = helixHeight / 2;

  geometry = new THREE.BufferGeometry();

  const positions = new Float32Array(PARTICLE_COUNT * 3);
  const sizes = new Float32Array(PARTICLE_COUNT);
  const colors = new Float32Array(PARTICLE_COUNT * 3);
  const alphas = new Float32Array(PARTICLE_COUNT);
  phases = new Float32Array(PARTICLE_COUNT * 4); // angularVel, floatPhase, pulsePhase, baseT

  const cyan = new THREE.Color(0x00e5ff);
  const purple = new THREE.Color(0x8000ff);
  const tmpColor = new THREE.Color();

  for (let i = 0; i < PARTICLE_COUNT; i++) {
    // Distribute along helix parameter
    const t = Math.random();
    const angle = t * turns * Math.PI * 2;
    // Pick random offset around the helix strand path
    const offsetAngle = Math.random() * Math.PI * 2;
    const offsetR = (Math.random() * 0.5 + 0.5) * particleSettings.scatterRadius;

    const baseX = helixRadius * Math.cos(angle);
    const baseZ = helixRadius * Math.sin(angle);
    const y = t * helixHeight - halfHeight;

    positions[i * 3] = baseX + Math.cos(offsetAngle) * offsetR;
    positions[i * 3 + 1] = y;
    positions[i * 3 + 2] = baseZ + Math.sin(offsetAngle) * offsetR;

    // Size: 1–4
    sizes[i] = 1.0 + Math.random() * 3.0;

    // Color: random blend of cyan and purple
    const blend = Math.random();
    tmpColor.copy(cyan).lerp(purple, blend);
    colors[i * 3] = tmpColor.r;
    colors[i * 3 + 1] = tmpColor.g;
    colors[i * 3 + 2] = tmpColor.b;

    // Alpha: fade near top/bottom edges
    const edgeDist = Math.min(t, 1.0 - t);
    alphas[i] = Math.min(edgeDist * 5.0, 1.0) * (0.3 + Math.random() * 0.7);

    // Phase data for animation
    phases[i * 4] = (0.2 + Math.random() * 0.8) * (Math.random() < 0.5 ? 1 : -1); // angularVel
    phases[i * 4 + 1] = Math.random() * Math.PI * 2; // floatPhase
    phases[i * 4 + 2] = Math.random() * Math.PI * 2; // pulsePhase
    phases[i * 4 + 3] = t; // baseT (position along helix)
  }

  geometry.setAttribute('position', new THREE.BufferAttribute(positions, 3));
  geometry.setAttribute('aSize', new THREE.BufferAttribute(sizes, 1));
  geometry.setAttribute('aColor', new THREE.BufferAttribute(colors, 3));
  geometry.setAttribute('aAlpha', new THREE.BufferAttribute(alphas, 1));

  const points = new THREE.Points(geometry, material);
  return points;
}

/**
 * Updates particle positions each frame.
 * @param {number} time — elapsed time in seconds
 * @param {{ helixRadius: number, helixHeight: number, turns: number }} config
 */
export function updateParticles(time, config) {
  if (!geometry || !phases) return;

  const { helixRadius, helixHeight, turns } = config;
  const halfHeight = helixHeight / 2;
  const positions = geometry.attributes.position.array;

  for (let i = 0; i < PARTICLE_COUNT; i++) {
    const angularVel = phases[i * 4];
    const floatPhase = phases[i * 4 + 1];
    const pulsePhase = phases[i * 4 + 2];
    const baseT = phases[i * 4 + 3];

    const angle = baseT * turns * Math.PI * 2 + angularVel * time * particleSettings.orbitSpeed;
    const pulse = Math.sin(time * particleSettings.pulseSpeed + pulsePhase) * particleSettings.pulseAmount;
    const r = helixRadius + particleSettings.scatterRadius * 0.6 + pulse;

    positions[i * 3] = r * Math.cos(angle);
    positions[i * 3 + 1] = baseT * helixHeight - halfHeight + Math.sin(time * particleSettings.floatSpeed + floatPhase) * particleSettings.floatAmount;
    positions[i * 3 + 2] = r * Math.sin(angle);
  }

  geometry.attributes.position.needsUpdate = true;
}
