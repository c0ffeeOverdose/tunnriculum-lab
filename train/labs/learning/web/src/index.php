<?php
require __DIR__ . '/config.php';
page_top('Home', 'home');
$featured = [find_product(1), find_product(2), find_product(10)];
?>
<section class="hero">
  <div class="hero-art">
    <svg viewBox="0 0 300 260" width="280" aria-hidden="true">
      <defs>
        <linearGradient id="hero-cup" x1="0" y1="0" x2="0" y2="1">
          <stop offset="0" stop-color="#FFE3EC"/><stop offset="1" stop-color="#FF9EBB"/>
        </linearGradient>
      </defs>
      <ellipse cx="150" cy="235" rx="95" ry="12" fill="#F3E3DE"/>
      <circle cx="52" cy="58" r="16" fill="#CDEBFA" opacity=".8"/>
      <circle cx="255" cy="96" r="11" fill="#CDEBFA" opacity=".8"/>
      <circle cx="238" cy="40" r="7" fill="#FFD9E3"/>
      <rect x="140" y="18" width="14" height="62" rx="7" fill="#8FCBE0" transform="rotate(10 147 45)"/>
      <path d="M95 74 h110 l-12 138 a18 18 0 0 1 -18 16 h-50 a18 18 0 0 1 -18 -16 z" fill="url(#hero-cup)"/>
      <path d="M95 74 h110 l-2 20 H97 z" fill="#ffffff" opacity=".45"/>
      <ellipse cx="150" cy="74" rx="57" ry="11" fill="#FFFDFB" stroke="#F3E3DE" stroke-width="2"/>
      <g fill="#5A4440"><circle cx="120" cy="195" r="9"/><circle cx="148" cy="203" r="9"/><circle cx="175" cy="194" r="9"/><circle cx="135" cy="180" r="9"/><circle cx="162" cy="178" r="9"/></g>
      <g stroke="#4A3B42" stroke-width="3.5" stroke-linecap="round" fill="none">
        <path d="M128 132 q5 5 10 0"/><path d="M160 132 q5 5 10 0"/>
      </g>
      <path d="M140 148 q9 9 19 0" stroke="#4A3B42" stroke-width="3.5" fill="none" stroke-linecap="round"/>
      <ellipse cx="116" cy="146" rx="8" ry="4.5" fill="#FF8FA8" opacity=".8"/>
      <ellipse cx="184" cy="146" rx="8" ry="4.5" fill="#FF8FA8" opacity=".8"/>
      <path d="M70 105 c-9 -11 -24 -2 -15 9 c6 8 15 13 15 13 s9 -5 15 -13 c9 -11 -6 -20 -15 -9z" fill="#FF6B8A"/>
      <path d="M232 130 c-7 -8 -19 -2 -12 7 c5 6 12 10 12 10 s7 -4 12 -10 c7 -9 -5 -15 -12 -7z" fill="#FF8FA8"/>
      <g fill="#fff"><path d="M66 150 l3 8 8 3 -8 3 -3 8 -3 -8 -8 -3 8 -3z"/><path d="M240 175 l2 6 6 2 -6 2 -2 6 -2 -6 -6 -2 6 -2z"/></g>
    </svg>
  </div>
  <div class="hero-text">
    <p class="kicker">Bubble tea, made with love</p>
    <h1>Strawberry-sweet sips &amp; the chewiest mochi pearls</h1>
    <p class="lead">Order online, skip the line. Every cup collects points toward free drinks.</p>
    <div class="hero-actions">
      <a class="btn" href="/menu.php">See the menu</a>
      <a class="btn ghost" href="/register.php">Join the club</a>
    </div>
  </div>
</section>

<section>
  <div class="section-head">
    <h2>Loved this week</h2>
    <a href="/menu.php">View all →</a>
  </div>
  <div class="grid">
    <?php foreach ($featured as $p): ?>
    <div class="card">
      <div class="card-art"><?= cup_svg($p, 84) ?></div>
      <h3><?= $p['name'] ?></h3>
      <p class="desc"><?= $p['desc'] ?></p>
      <div class="card-foot">
        <span class="price">฿<?= $p['price'] ?></span>
        <a class="btn small" href="/cart.php?add=<?= $p['id'] ?>">Add</a>
      </div>
    </div>
    <?php endforeach; ?>
  </div>
</section>

<section class="perks">
  <div class="perk"><span class="perk-icon">🍓</span><h3>Fresh daily</h3><p>Pearls cooked every 2 hours, fruit cut each morning</p></div>
  <div class="perk"><span class="perk-icon">🧋</span><h3>Points club</h3><p>Every baht earns points toward free drinks</p></div>
  <div class="perk"><span class="perk-icon">🛵</span><h3>Fast pickup</h3><p>Order ahead, grab and go — no queue</p></div>
</section>
<?php page_bottom(); ?>
