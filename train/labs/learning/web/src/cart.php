<?php
require __DIR__ . '/config.php';

if (isset($_GET['add'])) {
    $p = find_product((int)$_GET['add']);
    if ($p) $_SESSION['cart'][$p['id']] = ($_SESSION['cart'][$p['id']] ?? 0) + 1;
    if (isset($_GET['ajax'])) {
        header('Content-Type: application/json');
        echo json_encode(['success' => true, 'count' => array_sum($_SESSION['cart'])]);
        exit;
    }
    header('Location: /cart.php');
    exit;
}
if (isset($_GET['dec'])) {
    $id = (int)$_GET['dec'];
    if (isset($_SESSION['cart'][$id]) && --$_SESSION['cart'][$id] <= 0) unset($_SESSION['cart'][$id]);
    header('Location: /cart.php');
    exit;
}
if (isset($_GET['remove'])) {
    unset($_SESSION['cart'][(int)$_GET['remove']]);
    header('Location: /cart.php');
    exit;
}

page_top('Cart', 'cart');
$cart = $_SESSION['cart'] ?? [];
$total = 0;
?>
<h1>Your Cart 🧺</h1>
<?php if (!$cart): ?>
  <div class="empty">
    <p>Your cart is feeling light…</p>
    <a class="btn" href="/menu.php">Pick something yummy</a>
  </div>
<?php else: ?>
  <table class="cart-table">
    <tr><th></th><th>Drink</th><th>Qty</th><th>Price</th><th></th></tr>
    <?php foreach ($cart as $id => $qty):
        $p = find_product($id);
        if (!$p) continue;
        $line = $p['price'] * $qty;
        $total += $line; ?>
    <tr>
      <td class="mini-art"><?= cup_svg($p, 40) ?></td>
      <td><?= $p['name'] ?></td>
      <td class="qty">
        <a href="/cart.php?dec=<?= $id ?>">−</a>
        <span><?= $qty ?></span>
        <a href="/cart.php?add=<?= $id ?>">+</a>
      </td>
      <td>฿<?= $line ?></td>
      <td><a class="remove" href="/cart.php?remove=<?= $id ?>" title="Remove">✕</a></td>
    </tr>
    <?php endforeach; ?>
  </table>
  <div class="cart-total">
    <span>Total</span><strong>฿<?= $total ?></strong>
  </div>
  <div class="cart-actions">
    <a class="btn ghost" href="/menu.php">Add more</a>
    <a class="btn" href="/checkout.php">Checkout</a>
  </div>
<?php endif; ?>
<?php page_bottom(); ?>
