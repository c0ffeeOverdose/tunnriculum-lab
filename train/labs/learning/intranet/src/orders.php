<?php
require __DIR__ . '/config.php';
require_staff();
$orders = $db->query("SELECT o.id, o.total, o.status, o.created_at, m.username,
    GROUP_CONCAT(CONCAT(oi.product, ' x', oi.qty) ORDER BY oi.id SEPARATOR ', ') AS items
    FROM orders o
    JOIN members m ON m.id = o.member_id
    JOIN order_items oi ON oi.order_id = o.id
    GROUP BY o.id, o.total, o.status, o.created_at, m.username
    ORDER BY o.id DESC LIMIT 30");
$status = ['awaiting_slip' => 'Awaiting slip', 'slip_sent' => 'Verifying payment', 'paid' => 'Paid'];
$pill = ['awaiting_slip' => 'pill-await', 'slip_sent' => 'pill-verify', 'paid' => 'pill-paid'];
pos_top('Orders', 'orders.php');
?>
<h1>Recent Orders</h1>
<p class="muted">Online storefront orders, newest first.</p>
<table>
  <tr><th>#</th><th>Customer</th><th>Items</th><th>Total</th><th>Payment</th></tr>
  <?php if ($orders->num_rows): while ($order = $orders->fetch_assoc()): $st = $order['status']; ?>
  <tr><td>W-<?= str_pad((string)$order['id'], 4, '0', STR_PAD_LEFT) ?></td><td><?= h($order['username']) ?></td><td><?= h($order['items']) ?></td><td>฿<?= number_format((int)$order['total']) ?></td>
  <td><span class="pill <?= h($pill[$st] ?? 'pill-await') ?>"><?= h($status[$st] ?? $st) ?></span></td></tr>
  <?php endwhile; else: ?>
  <tr><td colspan="5">No online orders yet.</td></tr>
  <?php endif; ?>
</table>
<?php pos_bottom(); ?>
