import { useEffect, useRef } from 'react';
import * as THREE from 'three';

interface SpatialCanvasProps {
  presence: boolean;
  heartRate: number;
  amplitudes: number[];
  targetPos?: { x: number; y: number; z: number };
}

export const SpatialCanvas: React.FC<SpatialCanvasProps> = ({ heartRate, targetPos }) => {
  const mountRef = useRef<HTMLDivElement>(null);
  const humanGroupRef = useRef<THREE.Group | null>(null);
  const targetPosRef = useRef(targetPos);
  targetPosRef.current = targetPos;
  const particlesRef = useRef<THREE.Points | null>(null);

  useEffect(() => {
    const container = mountRef.current;
    if (!container) return;

    // Scene, Camera, Renderer
    const scene = new THREE.Scene();
    scene.fog = new THREE.FogExp2(0x07090e, 0.04);

    const camera = new THREE.PerspectiveCamera(50, container.clientWidth / container.clientHeight, 0.1, 100);
    camera.position.set(0, 3.5, 6.5);
    camera.lookAt(0, 0.5, 0);

    const renderer = new THREE.WebGLRenderer({ antialias: true, alpha: true });
    renderer.setSize(container.clientWidth, container.clientHeight);
    renderer.setPixelRatio(Math.min(window.devicePixelRatio, 2));
    container.appendChild(renderer.domElement);

    // Grid Floor (Sensor Range Zone)
    const gridHelper = new THREE.GridHelper(10, 20, 0x00f0ff, 0x162035);
    gridHelper.position.y = -0.5;
    scene.add(gridHelper);

    // RF Sensor Node Triangulation Pillars
    const nodeGeometry = new THREE.CylinderGeometry(0.08, 0.08, 1.2, 16);
    const nodeMaterial = new THREE.MeshBasicMaterial({ color: 0x00f0ff, wireframe: true });
    const positions = [
      [-3, 0.1, -3],
      [3, 0.1, -3],
      [-3, 0.1, 3],
      [3, 0.1, 3],
    ];

    positions.forEach((pos) => {
      const node = new THREE.Mesh(nodeGeometry, nodeMaterial);
      node.position.set(pos[0], pos[1], pos[2]);
      scene.add(node);

      // Blinking antenna emitter light
      const light = new THREE.PointLight(0x00f0ff, 1.5, 4);
      light.position.set(pos[0], pos[1] + 0.6, pos[2]);
      scene.add(light);
    });

    // 3D Human Skeletal Body (Reconstructed Voxel Mesh)
    const humanGroup = new THREE.Group();
    const sphereGeo = new THREE.SphereGeometry(0.12, 12, 12);
    const limbMat = new THREE.MeshStandardMaterial({
      color: 0x00ff88,
      emissive: 0x00ff88,
      emissiveIntensity: 0.6,
      roughness: 0.2,
    });

    // 17 Joints
    const joints: THREE.Mesh[] = [];
    for (let i = 0; i < 17; i++) {
      const joint = new THREE.Mesh(sphereGeo, limbMat);
      joints.push(joint);
      humanGroup.add(joint);
    }
    humanGroup.position.set(0, 0.6, 0);
    scene.add(humanGroup);
    humanGroupRef.current = humanGroup;

    // Ambient Multipath RF Particle Cloud
    const particleCount = 250;
    const particleGeo = new THREE.BufferGeometry();
    const particlePositions = new Float32Array(particleCount * 3);
    for (let i = 0; i < particleCount * 3; i += 3) {
      particlePositions[i] = (Math.random() - 0.5) * 8;
      particlePositions[i + 1] = Math.random() * 3 - 0.5;
      particlePositions[i + 2] = (Math.random() - 0.5) * 8;
    }
    particleGeo.setAttribute('position', new THREE.BufferAttribute(particlePositions, 3));
    const particleMat = new THREE.PointsMaterial({
      size: 0.04,
      color: 0x00f0ff,
      transparent: true,
      opacity: 0.5,
    });
    const particles = new THREE.Points(particleGeo, particleMat);
    scene.add(particles);
    particlesRef.current = particles;

    // Light
    const ambient = new THREE.AmbientLight(0xffffff, 0.4);
    scene.add(ambient);

    // Animation Loop
    let clock = new THREE.Clock();
    let animId: number;

    const animate = () => {
      animId = requestAnimationFrame(animate);
      const t = clock.getElapsedTime();

      // Floating micro-motion modulated by contactless heart rate & 3D resolved target position
      if (humanGroupRef.current) {
        const hrBps = (heartRate || 72) / 60;
        const pulse = Math.sin(t * hrBps * Math.PI * 2) * 0.04;
        const tp = targetPosRef.current;

        if (tp) {
          // Map from (0..6, 0..6) room space to (-3..3, -3..3) Three.js grid
          const targetX = tp.x - 3.0;
          const targetZ = tp.y - 3.0;
          const targetY = (tp.z || 1.0) - 0.4 + pulse;

          humanGroupRef.current.position.x += (targetX - humanGroupRef.current.position.x) * 0.08;
          humanGroupRef.current.position.z += (targetZ - humanGroupRef.current.position.z) * 0.08;
          humanGroupRef.current.position.y += (targetY - humanGroupRef.current.position.y) * 0.08;
        } else {
          humanGroupRef.current.position.y = 0.6 + pulse;
          humanGroupRef.current.position.x = Math.sin(t * 0.3) * 1.6;
          humanGroupRef.current.position.z = Math.cos(t * 0.3) * 1.6;
        }

        // Dynamic joint articulation
        joints.forEach((joint, idx) => {
          joint.position.set(
            Math.sin(t * 2 + idx * 0.5) * 0.2,
            idx * 0.09 - 0.7,
            Math.cos(t * 2 + idx * 0.5) * 0.15
          );
        });
      }

      // Rotate particles slowly
      if (particlesRef.current) {
        particlesRef.current.rotation.y = t * 0.05;
      }

      renderer.render(scene, camera);
    };
    animate();

    const handleResize = () => {
      if (!container) return;
      camera.aspect = container.clientWidth / container.clientHeight;
      camera.updateProjectionMatrix();
      renderer.setSize(container.clientWidth, container.clientHeight);
    };
    window.addEventListener('resize', handleResize);

    return () => {
      cancelAnimationFrame(animId);
      window.removeEventListener('resize', handleResize);
      renderer.dispose();
      if (container.contains(renderer.domElement)) {
        container.removeChild(renderer.domElement);
      }
    };
  }, []);

  return <div ref={mountRef} style={{ width: '100%', height: '100%', position: 'relative' }} />;
};
