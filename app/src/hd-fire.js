(() => {
  const defaultPalette = [[42, 8, 2], [200, 70, 15], [255, 160, 40], [255, 230, 120], [255, 255, 240]];
  const clamp = (value, min, max) => Math.max(min, Math.min(max, value));
  const colorAt = (palette, index) => palette?.[index] || defaultPalette[index];
  const rgba = (color, alpha) => `rgba(${color[0]}, ${color[1]}, ${color[2]}, ${alpha})`;

  class HDFireRenderer {
    constructor(canvas) {
      this.canvas = canvas;
      this.ctx = canvas.getContext('2d', { alpha: true });
      this.state = null;
      this.time = 0;
      this.lastFrame = 0;
      this.intensity = 0;
      this.particles = [];
      this.spawnCarry = 0;
      this.width = 1;
      this.height = 1;
      this.pixelRatio = 1;
      this.resizeObserver = new ResizeObserver(() => this.resize());
      this.resizeObserver.observe(canvas);
      this.resize();
      requestAnimationFrame((now) => this.draw(now));
    }

    setState(state) {
      if (state) this.state = state;
    }

    resize() {
      const bounds = this.canvas.getBoundingClientRect();
      this.width = Math.max(bounds.width, 1);
      this.height = Math.max(bounds.height, 1);
      this.pixelRatio = Math.min(window.devicePixelRatio || 1, 2);
      const width = Math.max(1, Math.round(this.width * this.pixelRatio));
      const height = Math.max(1, Math.round(this.height * this.pixelRatio));
      if (this.canvas.width !== width || this.canvas.height !== height) {
        this.canvas.width = width;
        this.canvas.height = height;
      }
      this.ctx.setTransform(this.pixelRatio, 0, 0, this.pixelRatio, 0, 0);
    }

    draw(now) {
      const state = this.state || {};
      const fire = state.fire || state;
      const palette = fire.palette || defaultPalette;
      const dt = this.lastFrame ? clamp((now - this.lastFrame) / 1000, 0, 0.05) : 0;
      this.lastFrame = now;
      const paused = Boolean(fire.animation_paused);
      const reduceMotion = Boolean(state.reduceMotion ?? state.reduce_motion);
      const motion = reduceMotion ? 0.22 : 1;
      if (!paused) this.time += dt * motion;
      this.intensity += (clamp(fire.intensity || 0, 0, 1) - this.intensity) * Math.min(1, dt * 7);

      const bounds = this.canvas.getBoundingClientRect();
      if (Math.abs(bounds.width - this.width) > 0.5 || Math.abs(bounds.height - this.height) > 0.5) this.resize();
      const ctx = this.ctx;
      ctx.setTransform(this.pixelRatio, 0, 0, this.pixelRatio, 0, 0);
      ctx.clearRect(0, 0, this.width, this.height);

      const phase = String(fire.phase || 'unlit').toLowerCase();
      const emberHeat = clamp(fire.ember_heat || 0, 0, 1);
      const flameVisible = phase === 'flame' && this.intensity > 0.015;
      const baseY = this.height * 0.79;
      this.drawGlow(ctx, palette, baseY, flameVisible ? this.intensity : emberHeat);
      this.drawLogs(ctx, palette, baseY, emberHeat);
      if (flameVisible) this.drawFlame(ctx, palette, baseY, this.intensity, motion);
      if (reduceMotion) {
        this.particles = [];
      } else if (!paused) {
        this.updateParticles(dt, flameVisible ? fire.spark_burst || 0 : 0, flameVisible ? this.intensity : 0, baseY, palette, flameVisible);
      }
      this.drawParticles(ctx, palette);
      requestAnimationFrame((next) => this.draw(next));
    }

    drawGlow(ctx, palette, baseY, strength) {
      if (strength <= 0.01) return;
      const glowRadius = Math.min(this.width * 0.38, this.height * 0.22);
      const glow = ctx.createRadialGradient(this.width / 2, baseY - this.height * 0.06, 1, this.width / 2, baseY - this.height * 0.02, glowRadius);
      glow.addColorStop(0, rgba(colorAt(palette, 2), 0.28 + strength * 0.14));
      glow.addColorStop(0.48, rgba(colorAt(palette, 1), 0.12 + strength * 0.08));
      glow.addColorStop(1, rgba(colorAt(palette, 0), 0));
      ctx.fillStyle = glow;
      ctx.fillRect(0, 0, this.width, this.height);
    }

    drawLogs(ctx, palette, baseY, emberHeat) {
      const centerX = this.width / 2;
      const logWidth = this.width * 0.47;
      const logHeight = Math.max(7, this.height * 0.055);
      ctx.save();
      ctx.lineCap = 'round';
      ctx.shadowColor = rgba(colorAt(palette, 1), 0.18 + emberHeat * 0.3);
      ctx.shadowBlur = this.height * (0.035 + emberHeat * 0.055);
      for (const angle of [-0.16, 0.16]) {
        ctx.save();
        ctx.translate(centerX, baseY + logHeight * 0.52);
        ctx.rotate(angle);
        const wood = ctx.createLinearGradient(0, -logHeight, 0, logHeight);
        wood.addColorStop(0, '#7e4a2b');
        wood.addColorStop(0.34, '#38241d');
        wood.addColorStop(0.72, '#211714');
        wood.addColorStop(1, '#100d0c');
        ctx.fillStyle = wood;
        ctx.beginPath();
        ctx.roundRect(-logWidth / 2, -logHeight / 2, logWidth, logHeight, logHeight / 2);
        ctx.fill();
        ctx.strokeStyle = 'rgba(255, 190, 112, 0.13)';
        ctx.lineWidth = Math.max(1, this.height * 0.004);
        ctx.beginPath();
        ctx.moveTo(-logWidth * 0.32, -logHeight * 0.12);
        ctx.quadraticCurveTo(0, -logHeight * 0.33, logWidth * 0.31, -logHeight * 0.1);
        ctx.stroke();
        ctx.restore();
      }
      ctx.restore();

      if (emberHeat > 0.03) {
        ctx.save();
        ctx.globalCompositeOperation = 'screen';
        ctx.fillStyle = rgba(colorAt(palette, 2), clamp(emberHeat * 0.72, 0, 0.72));
        for (let index = 0; index < 5; index += 1) {
          const x = centerX + (index - 2) * this.width * 0.055;
          ctx.beginPath();
          ctx.ellipse(x, baseY + logHeight * 0.16, this.width * 0.026, logHeight * 0.2, 0, 0, Math.PI * 2);
          ctx.fill();
        }
        ctx.restore();
      }
    }

    drawFlame(ctx, palette, baseY, intensity, motion) {
      const centerX = this.width / 2;
      const tokenActivity = Math.sqrt(clamp(intensity, 0, 1));
      const height = Math.min(this.height * (0.27 + tokenActivity * 0.56), baseY - this.height * 0.02);
      const width = this.width * (0.13 + tokenActivity * 0.23);
      const sway = (Math.sin(this.time * 2.8) + Math.sin(this.time * 5.3 + 0.8) * 0.35)
        * this.width * 0.045 * motion;
      const outer = ctx.createLinearGradient(centerX, baseY, centerX, baseY - height * 1.12);
      outer.addColorStop(0, rgba(colorAt(palette, 0), 0.76));
      outer.addColorStop(0.32, rgba(colorAt(palette, 1), 0.84));
      outer.addColorStop(0.72, rgba(colorAt(palette, 2), 0.48));
      outer.addColorStop(1, rgba(colorAt(palette, 2), 0));
      const mid = ctx.createLinearGradient(centerX, baseY, centerX, baseY - height * 0.92);
      mid.addColorStop(0, rgba(colorAt(palette, 1), 0.94));
      mid.addColorStop(0.45, rgba(colorAt(palette, 2), 0.9));
      mid.addColorStop(0.82, rgba(colorAt(palette, 3), 0.68));
      mid.addColorStop(1, rgba(colorAt(palette, 3), 0));
      const core = ctx.createLinearGradient(centerX, baseY, centerX, baseY - height * 0.64);
      core.addColorStop(0, rgba(colorAt(palette, 2), 0.95));
      core.addColorStop(0.62, rgba(colorAt(palette, 3), 0.86));
      core.addColorStop(1, rgba(colorAt(palette, 4), 0));
      const coolEdge = ctx.createLinearGradient(centerX, baseY, centerX, baseY - height * 0.86);
      coolEdge.addColorStop(0, 'rgba(22, 194, 205, 0.83)');
      coolEdge.addColorStop(0.38, 'rgba(13, 174, 205, 0.62)');
      coolEdge.addColorStop(0.78, 'rgba(20, 168, 220, 0.3)');
      coolEdge.addColorStop(1, 'rgba(20, 168, 220, 0)');

      ctx.save();
      ctx.globalCompositeOperation = 'screen';
      this.drawTongue(ctx, centerX, baseY, -width * 0.28, width * 0.52, height * 0.62, 4.3, 2.2, coolEdge, motion, sway * 0.7);
      this.drawTongue(ctx, centerX, baseY, width * 0.3, width * 0.46, height * 0.68, 4.8, 0.6, coolEdge, motion, sway * 0.7);
      this.drawTongue(ctx, centerX, baseY, -width * 0.52, width * 0.78, height * 0.75, 4.1, 0.2, outer, motion, sway);
      this.drawTongue(ctx, centerX, baseY, width * 0.5, width * 0.72, height * 0.81, 4.5, 2.4, outer, motion, sway);
      this.drawTongue(ctx, centerX, baseY, 0, width * 1.05, height * 0.98, 5.2, 1.1, outer, motion, sway * 0.35);
      this.drawTongue(ctx, centerX, baseY, -width * 0.3, width * 0.46, height * 0.65, 5.8, 0.8, mid, motion, sway);
      this.drawTongue(ctx, centerX, baseY, width * 0.26, width * 0.48, height * 0.7, 5.1, 2.8, mid, motion, sway);
      this.drawTongue(ctx, centerX, baseY, 0, width * 0.7, height * 0.79, 6.2, 0.4, mid, motion, sway * 0.5);
      this.drawTongue(ctx, centerX, baseY, 0, width * 0.42, height * 0.56, 6.8, 1.9, core, motion, sway * 0.15);

      const pulse = 0.9 + Math.sin(this.time * 8) * 0.1;
      const heart = ctx.createRadialGradient(centerX + sway * 0.2, baseY - height * 0.13, 1, centerX, baseY - height * 0.13, width * 0.46);
      heart.addColorStop(0, `rgba(255,255,245,${0.82 * pulse})`);
      heart.addColorStop(0.42, rgba(colorAt(palette, 4), 0.74 * pulse));
      heart.addColorStop(1, rgba(colorAt(palette, 3), 0));
      ctx.fillStyle = heart;
      ctx.beginPath();
      ctx.ellipse(centerX + sway * 0.2, baseY - height * 0.14, width * 0.29, height * 0.14, 0, 0, Math.PI * 2);
      ctx.fill();
      ctx.restore();
    }

    drawTongue(ctx, centerX, baseY, offset, width, height, speed, phase, gradient, motion, wind = 0) {
      const sway = Math.sin(this.time * speed + phase) * width * 0.3 * motion;
      const tipSway = Math.cos(this.time * speed * 1.28 + phase) * width * 0.38 * motion;
      const tipFlicker = (1 + Math.sin(this.time * speed * 1.6 + phase)) * height * 0.04 * motion;
      ctx.fillStyle = gradient;
      ctx.beginPath();
      ctx.moveTo(centerX + offset - width / 2, baseY);
      ctx.bezierCurveTo(centerX + offset - width * 0.7 + sway * 0.45 + wind * 0.28, baseY - height * 0.36,
        centerX + offset - width * 0.35 + sway + wind * 0.78, baseY - height * 0.72, centerX + offset + tipSway + wind, baseY - height + tipFlicker);
      ctx.bezierCurveTo(centerX + offset + width * 0.42 + sway + wind * 0.78, baseY - height * 0.7,
        centerX + offset + width * 0.72 + sway * 0.35 + wind * 0.28, baseY - height * 0.34, centerX + offset + width / 2, baseY);
      ctx.closePath();
      ctx.fill();
    }

    updateParticles(dt, sparkBurst, intensity, baseY, palette, flameVisible) {
      const burst = clamp(sparkBurst, 0, 1);
      this.spawnCarry += flameVisible ? dt * (2 + intensity * 11 + burst * 62) : 0;
      const maxParticles = flameVisible ? 18 + Math.round(intensity * 26 + burst * 15) : 0;
      while (this.spawnCarry >= 1 && this.particles.length < maxParticles) {
        this.spawnCarry -= 1;
        const life = 0.55 + Math.random() * 0.8;
        const smoke = Math.random() < 0.06;
        this.particles.push({
          x: this.width / 2 + (Math.random() - 0.5) * this.width * 0.26,
          y: baseY - this.height * 0.05,
          vx: (Math.random() - 0.5) * this.width * (smoke ? 0.06 : 0.11),
          vy: -(this.height * (smoke ? 0.07 + Math.random() * 0.05 : 0.2 + Math.random() * 0.23)),
          size: Math.max(1, this.width * (smoke ? 0.01 + Math.random() * 0.006 : 0.004 + Math.random() * 0.004)),
          life,
          maxLife: life,
          color: Math.random() < 0.32 ? 4 : Math.random() < 0.6 ? 3 : 2,
          smoke,
          phase: Math.random() * Math.PI * 2,
          alpha: 0.5,
        });
      }
      this.particles = this.particles.filter((particle) => {
        particle.life -= dt;
        if (particle.life <= 0 || particle.y < -this.height * 0.08) return false;
        const progress = 1 - particle.life / particle.maxLife;
        particle.x += (particle.vx + Math.sin(this.time * 3 + particle.y * 0.045 + particle.phase) * this.width * 0.016) * dt;
        particle.y += particle.vy * dt;
        particle.vy *= 0.99;
        particle.alpha = Math.sin(progress * Math.PI) * (particle.smoke ? 0.16 : 0.95);
        return true;
      });
    }

    drawParticles(ctx, palette) {
      if (!this.particles.length) return;
      ctx.save();
      ctx.globalCompositeOperation = 'screen';
      for (const particle of this.particles) {
        const radius = particle.size * (1 - (1 - particle.life / particle.maxLife) * 0.45);
        const color = particle.smoke ? [125, 118, 126] : colorAt(palette, particle.color);
        const glow = ctx.createRadialGradient(particle.x, particle.y, 0, particle.x, particle.y, radius * 2.5);
        glow.addColorStop(0, rgba(color, particle.alpha));
        glow.addColorStop(0.4, rgba(particle.smoke ? color : colorAt(palette, 2), particle.alpha * 0.6));
        glow.addColorStop(1, rgba(color, 0));
        ctx.fillStyle = glow;
        ctx.beginPath();
        ctx.arc(particle.x, particle.y, radius * 2.5, 0, Math.PI * 2);
        ctx.fill();
      }
      ctx.restore();
    }
  }

  window.HDFireRenderer = { attach: (canvas) => new HDFireRenderer(canvas) };
})();
