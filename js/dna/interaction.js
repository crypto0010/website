export const interactionSettings = {
  maxTilt: 0.26,
  lerpFactor: 0.05,
  rotationSpeed: 0.1,
  floatAmplitude: 0.2,
  floatSpeed: 0.5,
};

let mouse = { x: 0, y: 0 };
let target = { tiltX: 0, tiltZ: 0 };
let current = { tiltX: 0, tiltZ: 0 };
let group = null;
let baseRotationY = 0;

/**
 * Sets up mouse tracking and binds to the DNA group.
 * @param {THREE.Group} dnaGroup
 */
export function setupInteraction(dnaGroup) {
  group = dnaGroup;

  window.addEventListener('mousemove', (e) => {
    mouse.x = (e.clientX / window.innerWidth) * 2 - 1;
    mouse.y = (e.clientY / window.innerHeight) * 2 - 1;
    target.tiltX = -mouse.y * interactionSettings.maxTilt;
    target.tiltZ = mouse.x * interactionSettings.maxTilt;
  });

  window.addEventListener('mouseleave', () => {
    target.tiltX = 0;
    target.tiltZ = 0;
  });
}

/**
 * Updates group rotation each frame. Call from animation loop.
 * @param {number} time — elapsed seconds
 * @param {number} delta — seconds since last frame
 */
export function updateInteraction(time, delta) {
  if (!group) return;

  // Idle rotation
  baseRotationY += interactionSettings.rotationSpeed * delta;
  group.rotation.y = baseRotationY;

  // Lerp mouse tilt (frame-rate independent)
  const factor = 1.0 - Math.pow(1.0 - interactionSettings.lerpFactor, delta * 60);
  current.tiltX += (target.tiltX - current.tiltX) * factor;
  current.tiltZ += (target.tiltZ - current.tiltZ) * factor;
  group.rotation.x = current.tiltX;
  group.rotation.z = current.tiltZ;

  // Vertical float
  group.position.y = Math.sin(time * interactionSettings.floatSpeed) * interactionSettings.floatAmplitude;
}
