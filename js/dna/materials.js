import * as THREE from 'three';

/**
 * Creates a neon Fresnel ShaderMaterial for a DNA ribbon strand.
 * @param {THREE.Color|number} color — base neon color (e.g. 0x00e5ff)
 * @returns {THREE.ShaderMaterial}
 */
export function createStrandMaterial(color) {
  const c = new THREE.Color(color);

  return new THREE.ShaderMaterial({
    uniforms: {
      uColor: { value: c },
      uEmissiveIntensity: { value: 0.7 },
    },
    vertexShader: /* glsl */ `
      varying vec3 vNormal;
      varying vec3 vViewDir;

      void main() {
        vec4 mvPosition = modelViewMatrix * vec4(position, 1.0);
        vNormal = normalize(normalMatrix * normal);
        vViewDir = normalize(-mvPosition.xyz);
        gl_Position = projectionMatrix * mvPosition;
      }
    `,
    fragmentShader: /* glsl */ `
      uniform vec3 uColor;
      uniform float uEmissiveIntensity;

      varying vec3 vNormal;
      varying vec3 vViewDir;

      void main() {
        float fresnel = pow(1.0 - abs(dot(vNormal, vViewDir)), 3.0);
        vec3 emissive = uColor * uEmissiveIntensity;
        vec3 finalColor = mix(uColor * 0.3, emissive, fresnel);
        float alpha = (0.85 + fresnel * 0.15) * 0.5;
        gl_FragColor = vec4(finalColor, alpha);
      }
    `,
    transparent: true,
    side: THREE.DoubleSide,
  });
}

/**
 * Creates the rung material — standard emissive for bloom pickup.
 * @returns {THREE.MeshStandardMaterial}
 */
export function createRungMaterial() {
  return new THREE.MeshStandardMaterial({
    color: 0x00b8d4,
    emissive: 0x00b8d4,
    emissiveIntensity: 0.4,
    roughness: 0.3,
    metalness: 0.5,
    transparent: true,
    opacity: 0.5,
  });
}

/**
 * Creates a ShaderMaterial for the orbiting particle system.
 * @returns {THREE.ShaderMaterial}
 */
export function createParticleMaterial() {
  return new THREE.ShaderMaterial({
    uniforms: {
      uSizeScale: { value: 0.5 },
      uOpacity: { value: 1.0 },
    },
    vertexShader: /* glsl */ `
      uniform float uSizeScale;

      attribute float aSize;
      attribute vec3 aColor;
      attribute float aAlpha;

      varying vec3 vColor;
      varying float vAlpha;

      void main() {
        vColor = aColor;
        vAlpha = aAlpha;
        vec4 mvPosition = modelViewMatrix * vec4(position, 1.0);
        gl_PointSize = aSize * uSizeScale * (200.0 / -mvPosition.z);
        gl_Position = projectionMatrix * mvPosition;
      }
    `,
    fragmentShader: /* glsl */ `
      uniform float uOpacity;

      varying vec3 vColor;
      varying float vAlpha;

      void main() {
        float dist = length(gl_PointCoord - vec2(0.5));
        if (dist > 0.5) discard;
        float alpha = vAlpha * uOpacity * smoothstep(0.5, 0.1, dist);
        gl_FragColor = vec4(vColor, alpha);
      }
    `,
    transparent: true,
    blending: THREE.AdditiveBlending,
    depthWrite: false,
  });
}
