<?php
require __DIR__ . '/config.php';
$cats = ['All', 'Signature', 'Milk Tea', 'Fruit Tea', 'Smoothie'];
$cat = $_GET['cat'] ?? 'All';
if (!in_array($cat, $cats)) $cat = 'All';
page_top('Menu', 'menu');
?>
<div class="section-head">
  <h1>Our Menu</h1>
</div>
<div class="chips">
  <?php foreach ($cats as $c): ?>
    <a class="chip<?= $c === $cat ? ' on' : '' ?>" href="/menu.php?cat=<?= urlencode($c) ?>"><?= $c ?></a>
  <?php endforeach; ?>
</div>
<div class="grid">
  <?php foreach ($PRODUCTS as $p):
      if ($cat !== 'All' && $p['cat'] !== $cat) continue; ?>
  <div class="card">
    <div class="card-art"><?= cup_svg($p, 84) ?></div>
    <span class="tag"><?= $p['cat'] ?></span>
    <h3><?= $p['name'] ?></h3>
    <p class="desc"><?= $p['desc'] ?></p>
    <div class="card-foot">
      <span class="price">฿<?= $p['price'] ?></span>
      <a class="btn small" href="/cart.php?add=<?= $p['id'] ?>&cat=<?= urlencode($cat) ?>">Add</a>
    </div>
  </div>
  <?php endforeach; ?>
</div>
<?php page_bottom(); ?>
