<?php
require __DIR__ . '/config.php';
$u = current_user($db);
if (!$u) { header('Location: /login.php'); exit; }
$cart = $_SESSION['cart'] ?? [];
if (!$cart) { header('Location: /cart.php'); exit; }

$total = 0;
$items = [];
foreach ($cart as $id => $qty) {
    $p = find_product($id);
    if (!$p) continue;
    $items[] = [$p, $qty];
    $total += $p['price'] * $qty;
}

if ($_SERVER['REQUEST_METHOD'] === 'POST') {
    $stmt = $db->prepare("INSERT INTO orders (member_id, total) VALUES (?, ?)");
    $stmt->bind_param('ii', $u['id'], $total);
    $stmt->execute();
    $oid = $db->insert_id;
    $si = $db->prepare("INSERT INTO order_items (order_id, product, price, qty) VALUES (?, ?, ?, ?)");
    foreach ($items as [$p, $qty]) {
        $si->bind_param('isii', $oid, $p['name'], $p['price'], $qty);
        $si->execute();
    }
    unset($_SESSION['cart']);
    header("Location: /pay.php?order=$oid");
    exit;
}

page_top('Checkout', 'cart');
?>
<h1>Checkout</h1>
<div class="checkout">
  <div class="panel">
    <h2>Your drinks</h2>
    <table class="cart-table">
      <?php foreach ($items as [$p, $qty]): ?>
      <tr>
        <td class="mini-art"><?= cup_svg($p, 36) ?></td>
        <td><?= $p['name'] ?></td>
        <td>×<?= $qty ?></td>
        <td>฿<?= $p['price'] * $qty ?></td>
      </tr>
      <?php endforeach; ?>
    </table>
    <div class="cart-total"><span>Total</span><strong>฿<?= $total ?></strong></div>
  </div>
  <div class="panel">
    <h2>Pickup details</h2>
    <form method="post" class="stack">
      <label>Pickup branch
        <select name="branch">
          <option>Siam Square 01</option>
          <option>Ari</option>
          <option>Chiang Mai Nimman</option>
        </select>
      </label>
      <label>Pickup time
        <select name="ptime">
          <option>ASAP (about 15 min)</option>
          <option>In 30 minutes</option>
          <option>In 1 hour</option>
        </select>
      </label>
      <button class="btn" type="submit">Place order · ฿<?= $total ?></button>
      <p class="hint">Payment: PromptPay / bank transfer on the next page.</p>
    </form>
  </div>
</div>
<?php page_bottom(); ?>
