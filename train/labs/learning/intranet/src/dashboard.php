<?php
require __DIR__ . '/config.php';
require_staff();
$summary = $db->query("SELECT COUNT(*) AS orders_today, COALESCE(SUM(total), 0) AS revenue FROM orders WHERE DATE(created_at) = CURDATE()")->fetch_assoc();
$low = $db->query("SELECT COUNT(*) AS count FROM ingredients WHERE quantity <= reorder_at")->fetch_assoc();
$top = $db->query("SELECT product, SUM(qty) AS cups FROM order_items GROUP BY product ORDER BY cups DESC, product ASC LIMIT 3");
pos_top('Dashboard', 'dashboard.php');
?>
<h1>Today at Tang Wang Jeng</h1>
<p class="muted">Branch: Siam Square 01 · <?= date('D j M Y') ?></p>
<div class="cards">
  <div class="card"><h3>Online orders today</h3><p class="big"><?= (int)$summary['orders_today'] ?></p></div>
  <div class="card"><h3>Online revenue</h3><p class="big">฿<?= number_format((int)$summary['revenue']) ?></p></div>
  <div class="card"><h3>Low stock alerts</h3><p class="big <?= (int)$low['count'] ? 'warn' : '' ?>"><?= (int)$low['count'] ?></p></div>
</div>
<h2>Top sellers this week</h2>
<table>
  <tr><th>Drink</th><th>Cups ordered online</th></tr>
  <?php if ($top->num_rows): while ($row = $top->fetch_assoc()): ?>
  <tr><td><?= h($row['product']) ?></td><td><?= (int)$row['cups'] ?></td></tr>
  <?php endwhile; else: ?>
  <tr><td colspan="2">No online orders have arrived yet.</td></tr>
  <?php endif; ?>
</table>
<?php pos_bottom(); ?>
