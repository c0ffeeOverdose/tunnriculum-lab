<?php
require __DIR__ . '/config.php';
$msg = '';
if ($_SERVER['REQUEST_METHOD'] === 'POST') {
    // Staff login: user 'staff', password set by head office.
    if ($_POST['username'] === 'staff' && $_POST['password'] === $POS_PASS) {
        $_SESSION['staff'] = true;
        header('Location: /dashboard.php');
        exit;
    }
    $msg = 'Invalid staff credentials.';
}
?><!doctype html><html lang="en"><head><meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>Staff Login · Tang Wang Jeng</title><link rel="stylesheet" href="/style.css"></head>
<body class="login-page">
<form class="login-box" method="post">
  <div class="brand">
    <svg viewBox="0 0 24 24" width="46" height="46" aria-hidden="true">
      <path d="M7 4 h10 l-2 16 a2 2 0 0 1 -2 2 h-2 a2 2 0 0 1 -2 -2 z" fill="#FF9EBB"/>
      <rect x="11" y="1" width="2.4" height="6" rx="1.2" fill="#8FCBE0"/>
      <circle cx="10" cy="15" r="1.4" fill="#5A4440"/><circle cx="13.5" cy="16.5" r="1.4" fill="#5A4440"/>
    </svg>
    <div><strong>Tang Wang Jeng</strong><small>Staff POS</small></div>
  </div>
  <p class="sub">Tang Wang Jeng internal system — authorized staff only</p>
  <?php if ($msg) echo "<p class='error'>$msg</p>"; ?>
  <input name="username" placeholder="Username" autocomplete="username" required>
  <input name="password" type="password" placeholder="Password" autocomplete="current-password" required>
  <button type="submit">Sign in</button>
</form>
</body></html>
