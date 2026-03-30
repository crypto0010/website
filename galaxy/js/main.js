/* ==========================================================================
   Galaxy Hospital Bhopal — Shared JavaScript
   Three.js 3D scenes, GSAP animations, theme toggle, cursor, navigation
   ========================================================================== */

// ====== INIT ======
document.addEventListener('DOMContentLoaded', () => {
  lucide.createIcons();
  initCursor();
  initNav();
  initThemeToggle();
  initBackToTop();
  initSmoothScroll();
  initGSAP();
  initWebGLBackground();
  initFormHandler();
});

const prefersReduced = window.matchMedia('(prefers-reduced-motion: reduce)').matches;
const isDark = () => document.documentElement.getAttribute('data-theme') === 'dark';

// ====== THEME TOGGLE ======
function initThemeToggle() {
  const btn = document.getElementById('theme-toggle');
  if (!btn) return;
  btn.addEventListener('click', () => {
    const next = isDark() ? 'light' : 'dark';
    document.documentElement.setAttribute('data-theme', next);
    localStorage.setItem('galaxy-theme', next);
    if (window.updateWebGLTheme) window.updateWebGLTheme();
  });
}

// ====== CUSTOM CURSOR ======
function initCursor() {
  const dot = document.getElementById('cursor-dot');
  const ring = document.getElementById('cursor-ring');
  if (!dot || !ring || window.matchMedia('(pointer:coarse)').matches) return;

  let mx = 0, my = 0, dx = 0, dy = 0;
  document.addEventListener('mousemove', e => {
    mx = e.clientX; my = e.clientY;
    dot.style.left = mx - 4 + 'px';
    dot.style.top = my - 4 + 'px';
  });

  (function lerp() {
    dx += (mx - dx) * 0.12;
    dy += (my - dy) * 0.12;
    ring.style.left = dx - 20 + 'px';
    ring.style.top = dy - 20 + 'px';
    requestAnimationFrame(lerp);
  })();

  const hoverEls = 'a,button,.service-card,.service-card-img,.card,.testimonial-card,.bento-item,.contact-item,.about-badge,.img-card,select';
  document.querySelectorAll(hoverEls).forEach(el => {
    el.addEventListener('mouseenter', () => ring.classList.add('hovering'));
    el.addEventListener('mouseleave', () => ring.classList.remove('hovering'));
  });
}

// ====== NAV ======
function initNav() {
  const nav = document.getElementById('nav');
  if (!nav) return;

  window.addEventListener('scroll', () => nav.classList.toggle('scrolled', window.scrollY > 60));

  // Active link highlighting
  const sections = document.querySelectorAll('section[id]');
  const navAs = document.querySelectorAll('.nav-links a');
  const currentPage = window.location.pathname.split('/').pop() || 'index.html';

  // Highlight current page link
  navAs.forEach(a => {
    const href = a.getAttribute('href');
    if (href === currentPage || (currentPage === '' && href === 'index.html') || (currentPage === 'index.html' && href === 'index.html')) {
      // Don't override section-based active state on index
    }
  });

  if (sections.length > 0) {
    window.addEventListener('scroll', () => {
      let cur = '';
      sections.forEach(s => {
        if (s.id && window.scrollY >= s.offsetTop - 120) cur = s.id;
      });
      navAs.forEach(a => {
        const href = a.getAttribute('href');
        if (href.startsWith('#')) {
          a.classList.toggle('active', href === '#' + cur);
        }
      });
    });
  }

  // Mobile menu
  const openBtn = document.getElementById('mobile-open');
  const closeBtn = document.getElementById('mobile-close');
  const mobileNav = document.getElementById('mobile-nav');
  if (openBtn && closeBtn && mobileNav) {
    openBtn.addEventListener('click', () => mobileNav.classList.add('open'));
    closeBtn.addEventListener('click', () => mobileNav.classList.remove('open'));
    document.querySelectorAll('.mobile-link').forEach(l =>
      l.addEventListener('click', () => mobileNav.classList.remove('open'))
    );
  }
}

// ====== BACK TO TOP ======
function initBackToTop() {
  const btt = document.getElementById('btt');
  if (!btt) return;
  window.addEventListener('scroll', () => btt.classList.toggle('visible', window.scrollY > 600));
  btt.addEventListener('click', () => window.scrollTo({ top: 0, behavior: 'smooth' }));
}

// ====== SMOOTH SCROLL ======
function initSmoothScroll() {
  document.querySelectorAll('a[href^="#"]').forEach(a => {
    a.addEventListener('click', function (e) {
      const href = this.getAttribute('href');
      if (href === '#') return;
      const t = document.querySelector(href);
      if (t) {
        e.preventDefault();
        window.scrollTo({ top: t.offsetTop - 80, behavior: 'smooth' });
      }
    });
  });
}

// ====== GSAP ======
function initGSAP() {
  if (typeof gsap === 'undefined' || typeof ScrollTrigger === 'undefined') return;
  gsap.registerPlugin(ScrollTrigger);

  // Reveal-up animations
  document.querySelectorAll('.reveal-up').forEach(el => {
    gsap.to(el, {
      opacity: 1, y: 0, duration: 0.8, ease: 'power3.out',
      scrollTrigger: { trigger: el, start: 'top 88%', toggleActions: 'play none none none' }
    });
  });

  // Counter animations
  document.querySelectorAll('.counter').forEach(c => {
    const tgt = parseInt(c.dataset.target);
    ScrollTrigger.create({
      trigger: c, start: 'top 88%', once: true,
      onEnter: () => gsap.to({ v: 0 }, {
        v: tgt, duration: 2.5, ease: 'power2.out',
        onUpdate: function () {
          const v = Math.floor(this.targets()[0].v);
          c.textContent = v >= 1000 ? v.toLocaleString() : v;
        }
      })
    });
  });

  // Service card stagger
  const sGrid = document.querySelector('.services-grid');
  if (sGrid) {
    gsap.from('.service-card', {
      opacity: 0, y: 40, duration: 0.6, stagger: 0.08, ease: 'power3.out',
      scrollTrigger: { trigger: sGrid, start: 'top 85%' }
    });
  }

  // Card stagger
  const cards = document.querySelectorAll('.service-card-img, .doctors-grid .card');
  if (cards.length) {
    gsap.from(cards, {
      opacity: 0, y: 40, duration: 0.6, stagger: 0.1, ease: 'power3.out',
      scrollTrigger: { trigger: cards[0].parentElement, start: 'top 85%' }
    });
  }

  // Page hero text animation
  const heroTitle = document.querySelector('.page-hero-title');
  if (heroTitle) {
    gsap.from(heroTitle, { opacity: 0, y: 50, duration: 1, ease: 'power3.out', delay: 0.3 });
  }
  const heroEye = document.querySelector('.page-hero-eyebrow');
  if (heroEye) {
    gsap.from(heroEye, { opacity: 0, y: 20, duration: 0.8, ease: 'power3.out', delay: 0.1 });
  }
  const heroSub = document.querySelector('.page-hero-subtitle');
  if (heroSub) {
    gsap.from(heroSub, { opacity: 0, y: 30, duration: 0.8, ease: 'power3.out', delay: 0.5 });
  }
  const heroActions = document.querySelector('.page-hero-actions');
  if (heroActions) {
    gsap.from(heroActions, { opacity: 0, y: 30, duration: 0.8, ease: 'power3.out', delay: 0.7 });
  }
}

// ====== THREE.JS BACKGROUND ======
function initWebGLBackground() {
  const canvas = document.getElementById('webgl-bg');
  if (!canvas || typeof THREE === 'undefined') return;

  const renderer = new THREE.WebGLRenderer({ canvas, alpha: true, antialias: true });
  renderer.setSize(window.innerWidth, window.innerHeight);
  renderer.setPixelRatio(Math.min(window.devicePixelRatio, 1.5));

  const scene = new THREE.Scene();
  const camera = new THREE.PerspectiveCamera(60, window.innerWidth / window.innerHeight, 0.1, 500);
  camera.position.z = 50;

  // Particles
  const N = 400;
  const pPos = new Float32Array(N * 3);
  const pVel = [];
  for (let i = 0; i < N; i++) {
    pPos[i * 3] = (Math.random() - 0.5) * 80;
    pPos[i * 3 + 1] = (Math.random() - 0.5) * 80;
    pPos[i * 3 + 2] = (Math.random() - 0.5) * 40;
    pVel.push({ x: (Math.random() - 0.5) * 0.015, y: (Math.random() - 0.5) * 0.015, z: (Math.random() - 0.5) * 0.008 });
  }
  const pGeo = new THREE.BufferGeometry();
  pGeo.setAttribute('position', new THREE.BufferAttribute(pPos, 3));
  const pMat = new THREE.PointsMaterial({ size: 0.12, transparent: true, opacity: 0.4, blending: THREE.AdditiveBlending, sizeAttenuation: true });
  const particles = new THREE.Points(pGeo, pMat);
  scene.add(particles);

  // Connection lines
  const maxLines = 500;
  const linePos = new Float32Array(maxLines * 6);
  const lineGeo = new THREE.BufferGeometry();
  lineGeo.setAttribute('position', new THREE.BufferAttribute(linePos, 3));
  lineGeo.setDrawRange(0, 0);
  const lineMat = new THREE.LineBasicMaterial({ transparent: true, opacity: 0.05, blending: THREE.AdditiveBlending });
  const lines = new THREE.LineSegments(lineGeo, lineMat);
  scene.add(lines);

  // Wireframe meshes
  const meshes = [];
  const tk = new THREE.Mesh(
    new THREE.TorusKnotGeometry(9, 2.8, 120, 16, 2, 3),
    new THREE.MeshBasicMaterial({ wireframe: true, transparent: true, opacity: 0.05 })
  );
  tk.position.set(22, 5, -18);
  scene.add(tk); meshes.push(tk);

  const dod = new THREE.Mesh(
    new THREE.DodecahedronGeometry(5, 0),
    new THREE.MeshBasicMaterial({ wireframe: true, transparent: true, opacity: 0.04 })
  );
  dod.position.set(-20, -8, -12);
  scene.add(dod); meshes.push(dod);

  const oct = new THREE.Mesh(
    new THREE.OctahedronGeometry(3.5, 0),
    new THREE.MeshBasicMaterial({ wireframe: true, transparent: true, opacity: 0.06 })
  );
  oct.position.set(-8, 18, -8);
  scene.add(oct); meshes.push(oct);

  const torus = new THREE.Mesh(
    new THREE.TorusGeometry(13, 0.25, 16, 80),
    new THREE.MeshBasicMaterial({ transparent: true, opacity: 0.03 })
  );
  torus.rotation.x = Math.PI / 3;
  torus.position.z = -22;
  scene.add(torus); meshes.push(torus);

  // Icosahedron ring
  const icoRing = new THREE.Group();
  for (let i = 0; i < 6; i++) {
    const a = (i / 6) * Math.PI * 2;
    const ico = new THREE.Mesh(
      new THREE.IcosahedronGeometry(1, 0),
      new THREE.MeshBasicMaterial({ wireframe: true, transparent: true, opacity: 0.07 })
    );
    ico.position.set(Math.cos(a) * 16, Math.sin(a) * 16, -5);
    icoRing.add(ico); meshes.push(ico);
  }
  scene.add(icoRing);

  let mx = 0, my = 0;
  document.addEventListener('mousemove', e => {
    mx = (e.clientX / window.innerWidth - 0.5) * 2;
    my = (e.clientY / window.innerHeight - 0.5) * 2;
  });

  function updateColors() {
    const dark = isDark();
    const accent = dark ? 0xe8562a : 0xd14a1f;
    const gold = dark ? 0xc9a84c : 0xa07c28;
    pMat.color.set(dark ? 0x888888 : 0x999999);
    pMat.opacity = dark ? 0.4 : 0.18;
    lineMat.opacity = dark ? 0.05 : 0.025;
    tk.material.color.set(accent); tk.material.opacity = dark ? 0.05 : 0.04;
    dod.material.color.set(gold); dod.material.opacity = dark ? 0.04 : 0.03;
    oct.material.color.set(accent); oct.material.opacity = dark ? 0.06 : 0.04;
    torus.material.color.set(gold); torus.material.opacity = dark ? 0.03 : 0.02;
    icoRing.children.forEach((c, i) => {
      c.material.color.set(i % 2 === 0 ? accent : gold);
      c.material.opacity = dark ? 0.07 : 0.04;
    });
  }
  updateColors();
  window.updateWebGLTheme = updateColors;

  function animate() {
    requestAnimationFrame(animate);
    if (prefersReduced) { renderer.render(scene, camera); return; }

    const t = Date.now() * 0.001;
    const pos = particles.geometry.attributes.position.array;

    for (let i = 0; i < N; i++) {
      pos[i * 3] += pVel[i].x;
      pos[i * 3 + 1] += pVel[i].y;
      pos[i * 3 + 2] += pVel[i].z;
      if (pos[i * 3] > 40) pos[i * 3] = -40;
      if (pos[i * 3] < -40) pos[i * 3] = 40;
      if (pos[i * 3 + 1] > 40) pos[i * 3 + 1] = -40;
      if (pos[i * 3 + 1] < -40) pos[i * 3 + 1] = 40;
    }
    particles.geometry.attributes.position.needsUpdate = true;

    let li = 0;
    const lArr = lines.geometry.attributes.position.array;
    for (let i = 0; i < N && li < maxLines; i++) {
      for (let j = i + 1; j < N && li < maxLines; j++) {
        const dx = pos[i * 3] - pos[j * 3], dy = pos[i * 3 + 1] - pos[j * 3 + 1], dz = pos[i * 3 + 2] - pos[j * 3 + 2];
        if (dx * dx + dy * dy + dz * dz < 64) {
          const idx = li * 6;
          lArr[idx] = pos[i * 3]; lArr[idx + 1] = pos[i * 3 + 1]; lArr[idx + 2] = pos[i * 3 + 2];
          lArr[idx + 3] = pos[j * 3]; lArr[idx + 4] = pos[j * 3 + 1]; lArr[idx + 5] = pos[j * 3 + 2];
          li++;
        }
      }
    }
    lines.geometry.setDrawRange(0, li * 2);
    lines.geometry.attributes.position.needsUpdate = true;

    tk.rotation.x += 0.001; tk.rotation.y += 0.0015;
    dod.rotation.x -= 0.0008; dod.rotation.y += 0.001;
    oct.rotation.x += 0.002; oct.rotation.z += 0.001;
    icoRing.rotation.z += 0.0004;
    torus.rotation.z += 0.0005;
    const s = 1 + Math.sin(t * 0.3) * 0.06;
    tk.scale.set(s, s, s);

    camera.position.x += (mx * 2.5 - camera.position.x) * 0.012;
    camera.position.y += (-my * 2.5 - camera.position.y) * 0.012;
    camera.lookAt(0, 0, 0);

    renderer.render(scene, camera);
  }
  animate();

  window.addEventListener('resize', () => {
    camera.aspect = window.innerWidth / window.innerHeight;
    camera.updateProjectionMatrix();
    renderer.setSize(window.innerWidth, window.innerHeight);
  });
}

// ====== FORM HANDLER ======
function initFormHandler() {
  const form = document.getElementById('appointment-form');
  if (!form) return;
  form.addEventListener('submit', e => {
    e.preventDefault();
    const btn = document.getElementById('submit-btn');
    if (!btn) return;
    const orig = btn.innerHTML;
    btn.innerHTML = '<span>Sending...</span>';
    btn.disabled = true; btn.style.opacity = '0.6';
    setTimeout(() => {
      btn.innerHTML = '<span>Appointment Booked!</span>';
      btn.style.background = '#059669'; btn.style.opacity = '1';
      setTimeout(() => {
        btn.innerHTML = orig; btn.disabled = false; btn.style.background = '';
        lucide.createIcons();
      }, 3000);
    }, 2000);
  });
}
