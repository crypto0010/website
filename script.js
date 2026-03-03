// ============================================
// CSR & MSME Funding Opportunities — Slide Nav + Theme Toggle
// ============================================

(function () {
    // ========== VANTA BACKGROUND ==========
    let vantaEffect = null;
    let currentTheme = 'dark';

    function initVanta(theme) {
        // Destroy existing effect
        if (vantaEffect) {
            vantaEffect.destroy();
            vantaEffect = null;
        }

        const el = document.getElementById('slidesContainer');
        if (!el) return;

        if (theme === 'dark' && typeof VANTA !== 'undefined' && VANTA.TOPOLOGY) {
            vantaEffect = VANTA.TOPOLOGY({
                el: el,
                mouseControls: true,
                touchControls: true,
                gyroControls: false,
                minHeight: 200,
                minWidth: 200,
                scale: 1.0,
                scaleMobile: 1.0,
                color: 0x00d4ff,
                backgroundColor: 0x0a0e1a
            });
        } else if (theme === 'light' && typeof VANTA !== 'undefined' && VANTA.TRUNK) {
            vantaEffect = VANTA.TRUNK({
                el: el,
                mouseControls: true,
                touchControls: true,
                gyroControls: false,
                minHeight: 200,
                minWidth: 200,
                scale: 1.0,
                scaleMobile: 1.0,
                color: 0x7c3aed,
                backgroundColor: 0xf5f3ef,
                spacing: 0,
                chaos: 1.5
            });
        }
    }

    // ========== THEME TOGGLE ==========
    const themeToggle = document.getElementById('themeToggle');

    function setTheme(theme) {
        currentTheme = theme;
        document.documentElement.setAttribute('data-theme', theme);
        localStorage.setItem('pres-theme', theme);

        // Update QR code colors
        const qrImg = document.querySelector('.qr-img');
        if (qrImg) {
            const qrColor = theme === 'dark' ? '00d4ff' : '7c3aed';
            const qrBg = theme === 'dark' ? '0a0e1a' : 'ffffff';
            qrImg.src = `https://api.qrserver.com/v1/create-qr-code/?size=200x200&data=mailto:hemrajshobharamlamkuche@vitbhopal.ac.in&color=${qrColor}&bgcolor=${qrBg}&format=svg`;
        }

        initVanta(theme);
    }

    if (themeToggle) {
        themeToggle.addEventListener('click', () => {
            setTheme(currentTheme === 'dark' ? 'light' : 'dark');
        });
    }

    // Load saved theme or default to dark
    const savedTheme = localStorage.getItem('pres-theme') || 'dark';
    setTheme(savedTheme);

    // ========== SLIDE NAVIGATION ==========
    const slides = document.querySelectorAll('.slide');
    const totalSlides = slides.length;
    let currentSlide = 0;
    let isAnimating = false;

    const prevBtn = document.getElementById('prevBtn');
    const nextBtn = document.getElementById('nextBtn');
    const progressFill = document.getElementById('progressFill');
    const slideCounter = document.getElementById('slideCounter');
    const dotsContainer = document.getElementById('slideDots');
    const keyHint = document.getElementById('keyHint');

    // Build dots
    for (let i = 0; i < totalSlides; i++) {
        const dot = document.createElement('div');
        dot.classList.add('dot');
        if (i === 0) dot.classList.add('active');
        dot.addEventListener('click', () => goToSlide(i));
        dotsContainer.appendChild(dot);
    }
    const dots = dotsContainer.querySelectorAll('.dot');

    function updateUI() {
        slideCounter.querySelector('.current-slide').textContent =
            String(currentSlide + 1).padStart(2, '0');
        progressFill.style.width = ((currentSlide) / (totalSlides - 1) * 100) + '%';
        prevBtn.disabled = currentSlide === 0;
        nextBtn.disabled = currentSlide === totalSlides - 1;
        dots.forEach((d, i) => d.classList.toggle('active', i === currentSlide));
    }

    function goToSlide(index) {
        if (index === currentSlide || isAnimating || index < 0 || index >= totalSlides) return;
        isAnimating = true;

        const direction = index > currentSlide ? 1 : -1;
        const currentEl = slides[currentSlide];
        const nextEl = slides[index];

        nextEl.style.transition = 'none';
        nextEl.style.transform = `translateX(${direction * 60}px)`;
        nextEl.style.opacity = '0';
        nextEl.classList.add('active');
        nextEl.style.visibility = 'visible';

        void nextEl.offsetWidth;

        currentEl.style.transition = 'opacity 0.5s cubic-bezier(0.4,0,0.2,1), transform 0.5s cubic-bezier(0.4,0,0.2,1)';
        currentEl.style.transform = `translateX(${-direction * 60}px)`;
        currentEl.style.opacity = '0';

        nextEl.style.transition = 'opacity 0.5s cubic-bezier(0.4,0,0.2,1), transform 0.5s cubic-bezier(0.4,0,0.2,1)';
        nextEl.style.transform = 'translateX(0)';
        nextEl.style.opacity = '1';

        setTimeout(() => {
            currentEl.classList.remove('active');
            currentEl.style.visibility = 'hidden';
            currentSlide = index;
            updateUI();
            isAnimating = false;
        }, 520);
    }

    window.goToSlide = goToSlide;

    prevBtn.addEventListener('click', () => goToSlide(currentSlide - 1));
    nextBtn.addEventListener('click', () => goToSlide(currentSlide + 1));

    // Keyboard
    document.addEventListener('keydown', (e) => {
        if (keyHint && !keyHint.classList.contains('hidden')) {
            keyHint.classList.add('hidden');
        }
        switch (e.key) {
            case 'ArrowRight': case 'ArrowDown': case ' ':
                e.preventDefault(); goToSlide(currentSlide + 1); break;
            case 'ArrowLeft': case 'ArrowUp':
                e.preventDefault(); goToSlide(currentSlide - 1); break;
            case 'Home':
                e.preventDefault(); goToSlide(0); break;
            case 'End':
                e.preventDefault(); goToSlide(totalSlides - 1); break;
        }
    });

    // Touch swipe
    let touchStartX = 0, touchStartY = 0;
    document.addEventListener('touchstart', (e) => {
        touchStartX = e.changedTouches[0].screenX;
        touchStartY = e.changedTouches[0].screenY;
    }, { passive: true });

    document.addEventListener('touchend', (e) => {
        const dx = e.changedTouches[0].screenX - touchStartX;
        const dy = e.changedTouches[0].screenY - touchStartY;
        if (Math.abs(dx) > Math.abs(dy) && Math.abs(dx) > 50) {
            goToSlide(dx < 0 ? currentSlide + 1 : currentSlide - 1);
        }
    }, { passive: true });

    // Auto-hide hint
    setTimeout(() => {
        if (keyHint) keyHint.classList.add('hidden');
    }, 6000);

    // Init slides
    slides.forEach((s, i) => {
        if (i !== 0) { s.style.visibility = 'hidden'; s.style.opacity = '0'; }
    });
    updateUI();
})();
