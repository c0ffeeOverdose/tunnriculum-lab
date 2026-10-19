<?php
require __DIR__ . '/config.php';
$u = current_user($db);
if (!$u) { header('Location: /login.php'); exit; }
page_top('Orders', 'orders');

$stmt = $db->prepare("SELECT * FROM orders WHERE member_id = ? ORDER BY id DESC");
$stmt->bind_param('i', $u['id']);
$stmt->execute();
$orders = $stmt->get_result()->fetch_all(MYSQLI_ASSOC);
$status_label = ['awaiting_slip' => 'Waiting for slip', 'slip_sent' => 'Verifying', 'paid' => 'Paid'];
?>
<h1>My Orders</h1>
<?php if (!$orders): ?>
  <div class="empty">
    <p>No orders yet — your boba journey starts here.</p>
    <a class="btn" href="/menu.php">Browse menu</a>
  </div>
<?php else: ?>
  <table class="cart-table">
    <tr><th>Order</th><th>Date</th><th>Total</th><th>Status</th><th></th></tr>
    <?php foreach ($orders as $o): ?>
    <tr>
      <td>#<?= $o['id'] ?></td>
      <td><?= date('j M, H:i', strtotime($o['created_at'])) ?></td>
      <td>฿<?= (int)$o['total'] ?></td>
      <td><span class="status-pill"><?= $status_label[$o['status']] ?? $o['status'] ?></span></td>
      <td><?php if ($o['status'] === 'awaiting_slip'): ?>
        <a class="btn small" href="/pay.php?order=<?= $o['id'] ?>">Pay</a>
      <?php endif; ?></td>
    </tr>
    <?php endforeach; ?>
  </table>
<?php endif; ?>
<?php page_bottom(); ?>
