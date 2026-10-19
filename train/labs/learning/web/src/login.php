<?php
require __DIR__ . '/config.php';
$msg = '';
if ($_SERVER['REQUEST_METHOD'] === 'POST') {
    $stmt = $db->prepare("SELECT * FROM members WHERE username = ?");
    $stmt->bind_param('s', $_POST['username']);
    $stmt->execute();
    $u = $stmt->get_result()->fetch_assoc();
    if ($u && password_verify($_POST['password'], $u['password'])) {
        $_SESSION['uid'] = $u['id'];
        header('Location: /profile.php');
        exit;
    }
    $msg = 'Wrong username or password.';
}
page_top('Login');
?>
<div class="form-card">
  <h1>Welcome back 🧋</h1>
  <?php if ($msg) echo "<p class='error'>$msg</p>"; ?>
  <form method="post">
    <label>Username <input name="username" required></label>
    <label>Password <input name="password" type="password" required></label>
    <button class="btn" type="submit">Login</button>
  </form>
  <p>New to the club? <a href="/register.php">Sign up here</a></p>
</div>
<?php page_bottom(); ?>
