<?php
require __DIR__ . '/config.php';
$u = current_user($db);
if (!$u) { header('Location: /login.php'); exit; }
page_top('Account', 'profile');
?>
<div class="profile">
  <div class="avatar-ring">
    <svg viewBox="0 0 100 100" width="72" aria-hidden="true">
      <circle cx="50" cy="50" r="46" fill="#CDEBFA"/>
      <ellipse cx="50" cy="88" rx="30" ry="18" fill="#FF9EBB"/>
      <circle cx="50" cy="44" r="22" fill="#FFF6EC"/>
      <circle cx="35" cy="30" r="8" fill="#FFF6EC"/><circle cx="65" cy="30" r="8" fill="#FFF6EC"/>
      <circle cx="35" cy="31" r="4" fill="#FFC7D3"/><circle cx="65" cy="31" r="4" fill="#FFC7D3"/>
      <circle cx="42" cy="44" r="3" fill="#4A3B42"/><circle cx="58" cy="44" r="3" fill="#4A3B42"/>
      <ellipse cx="50" cy="52" rx="4" ry="3" fill="#FFC7D3"/>
      <path d="M46 58 q4 3 8 0" stroke="#4A3B42" stroke-width="2" fill="none" stroke-linecap="round"/>
    </svg>
  </div>
  <h1><?= htmlspecialchars($u['username']) ?></h1>
  <p class="points"><?= (int)$u['points'] ?> points</p>
  <p class="hint">Earn 1 point per ฿10 — 100 points = a free Signature drink.</p>
  <div class="hero-actions">
    <a class="btn" href="/menu.php">Order now</a>
    <a class="btn ghost" href="/orders.php">My orders</a>
  </div>
</div>
<?php page_bottom(); ?>
