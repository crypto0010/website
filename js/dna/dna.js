import * as THREE from 'three';

const DNA_CONFIG = {
  helixRadius: 2.0,
  helixHeight: 10.0,
  turns: 3.5,
  tubeRadius: 0.15,
  rungCount: 24,
  rungRadius: 0.04,
  segments: 256,
};

/**
 * Generates points along a helical path.
 * @param {number} phaseOffset — radians offset (0 for strand A, PI for strand B)
 * @returns {THREE.Vector3[]}
 */
function generateHelixPoints(phaseOffset) {
  const { helixRadius, helixHeight, turns, segments } = DNA_CONFIG;
  const points = [];
  const halfHeight = helixHeight / 2;

  for (let i = 0; i <= segments; i++) {
    const t = i / segments;
    const angle = t * turns * Math.PI * 2 + phaseOffset;
    const x = helixRadius * Math.cos(angle);
    const z = helixRadius * Math.sin(angle);
    const y = t * helixHeight - halfHeight;
    points.push(new THREE.Vector3(x, y, z));
  }

  return points;
}

/**
 * Creates the full DNA helix group.
 * @param {{ strandA: THREE.Material, strandB: THREE.Material, rung: THREE.Material }} materials
 * @returns {{ group: THREE.Group, config: typeof DNA_CONFIG }}
 */
export function createDNA(materials) {
  const group = new THREE.Group();
  const { tubeRadius, rungCount, rungRadius, segments } = DNA_CONFIG;

  // Strand A
  const pointsA = generateHelixPoints(0);
  const curveA = new THREE.CatmullRomCurve3(pointsA);
  const tubeA = new THREE.TubeGeometry(curveA, segments, tubeRadius, 12, false);
  const meshA = new THREE.Mesh(tubeA, materials.strandA);
  group.add(meshA);

  // Strand B
  const pointsB = generateHelixPoints(Math.PI);
  const curveB = new THREE.CatmullRomCurve3(pointsB);
  const tubeB = new THREE.TubeGeometry(curveB, segments, tubeRadius, 12, false);
  const meshB = new THREE.Mesh(tubeB, materials.strandB);
  group.add(meshB);

  // Rungs — instanced cylinders
  const rungGeometry = new THREE.CylinderGeometry(rungRadius, rungRadius, 1, 8);
  rungGeometry.rotateZ(Math.PI / 2); // align along X so we can scale length
  const rungMesh = new THREE.InstancedMesh(rungGeometry, materials.rung, rungCount);

  const dummy = new THREE.Object3D();

  for (let i = 0; i < rungCount; i++) {
    const t = (i + 0.5) / rungCount;
    const idx = Math.round(t * segments);

    const pA = pointsA[idx];
    const pB = pointsB[idx];

    // Position at midpoint
    const mid = new THREE.Vector3().addVectors(pA, pB).multiplyScalar(0.5);
    dummy.position.copy(mid);

    // Orient toward pB from pA
    const dir = new THREE.Vector3().subVectors(pB, pA);
    const length = dir.length();
    dir.normalize();

    dummy.quaternion.setFromUnitVectors(new THREE.Vector3(1, 0, 0), dir);
    dummy.scale.set(length, 1, 1);

    dummy.updateMatrix();
    rungMesh.setMatrixAt(i, dummy.matrix);
  }

  rungMesh.instanceMatrix.needsUpdate = true;
  group.add(rungMesh);

  return { group, config: DNA_CONFIG };
}
