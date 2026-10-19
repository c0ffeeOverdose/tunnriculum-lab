<?php
require __DIR__ . '/config.php';
require_staff();
$stock = $db->query("SELECT name, quantity, unit, reorder_at FROM ingredients ORDER BY name");
pos_top('Stock', 'stock.php');
?>
<h1>Ingredient Stock</h1>
<table>
  <tr><th>Item</th><th>On hand</th><th>Unit</th><th>Status</th></tr>
  <?php while ($item = $stock->fetch_assoc()): $isLow = (float)$item['quantity'] <= (float)$item['reorder_at']; ?>
  <tr<?= $isLow ? " class='low'" : '' ?>><td><?= h($item['name']) ?></td><td><?= rtrim(rtrim(number_format((float)$item['quantity'], 2, '.', ''), '0'), '.') ?></td><td><?= h($item['unit']) ?></td><td><?= $isLow ? 'LOW — reorder' : 'ok' ?></td></tr>
  <?php endwhile; ?>
</table>
<?php pos_bottom(); ?>
