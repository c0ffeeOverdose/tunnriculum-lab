<?php
require __DIR__ . '/config.php';
$msg = '';
if ($_SERVER['REQUEST_METHOD'] === 'POST') {
    $u = trim($_POST['username'] ?? '');
    $p = $_POST['password'] ?? '';
    if ($u === '' || $p === '') {
        $msg = 'Please fill in all fields.';
    } else {
        $stmt = $db->prepare("INSERT INTO members (username, password) VALUES (?, ?)");
        $hash = password_hash($p, PASSWORD_DEFAULT);
        $stmt->bind_param('ss', $u, $hash);
        if ($stmt->execute()) {
            $_SESSION['uid'] = $db->insert_id;
            header('Location: /profile.php');
            exit;
        }
        $msg = 'That username is taken.';
    }
}
page_top('Join us');
?>
<div class="form-card">
  <h1>Join the Tang Wang Jeng ChaNom Club 🐻</h1>
  <?php if ($msg) echo "<p class='error'>$msg</p>"; ?>
  <form method="post">
    <label>Username <input name="username" required></label>
    <label>Password <input name="password" type="password" required></label>
    <button class="btn" type="submit">Sign up</button>
  </form>
  <p>Already a member? <a href="/login.php">Login</a></p>
</div>
<?php page_bottom(); ?>
