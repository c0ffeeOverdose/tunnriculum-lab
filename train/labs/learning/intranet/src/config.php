<?php
// Tang Wang Jeng Intranet — staff POS backend. Internal use only.
$POS_PASS = getenv('POS_PASS') ?: 'mochi';
$HQ_HOST  = getenv('HQ_HOST') ?: 'hq.tangwangjeng.corp';
$DB_HOST = getenv('DB_HOST') ?: 'db';
$DB_NAME = getenv('DB_NAME') ?: 'chanom';
$DB_USER = getenv('DB_USER') ?: 'chanom';
$DB_PASS = getenv('DB_PASS') ?: 'chanom';

$db = new mysqli($DB_HOST, $DB_USER, $DB_PASS, $DB_NAME);
if ($db->connect_error) {
    die('The order database is unavailable. Please contact the branch manager.');
}
$db->set_charset('utf8mb4');

session_start();

function require_staff() {
    if (empty($_SESSION['staff'])) {
        header('Location: /login.php');
        exit;
    }
}

function pos_top($title, $current = '') {
    $logo = "<svg viewBox='0 0 24 24' width='32' height='32' aria-hidden='true'>
      <path d='M7 4 h10 l-2 16 a2 2 0 0 1 -2 2 h-2 a2 2 0 0 1 -2 -2 z' fill='#FF9EBB'/>
      <rect x='11' y='1' width='2.4' height='6' rx='1.2' fill='#8FCBE0'/>
      <circle cx='10' cy='15' r='1.4' fill='#5A4440'/><circle cx='13.5' cy='16.5' r='1.4' fill='#5A4440'/></svg>";
    $items = [
        'dashboard.php' => 'Dashboard',
        'stock.php' => 'Stock',
        'orders.php' => 'Orders',
        'recipes.php' => 'Recipes',
        'wiki.php' => 'IT Handbook',
    ];
    $nav = '';
    foreach ($items as $page => $label) {
        $nav .= "<a href='/$page'" . ($current === $page ? " class='active'" : '') . ">$label</a>";
    }
    echo "<!doctype html><html lang='en'><head><meta charset='utf-8'>
    <meta name='viewport' content='width=device-width, initial-scale=1'>
    <title>$title · Tang Wang Jeng Staff POS</title>
    <link rel='stylesheet' href='/style.css'></head><body>
    <aside class='side'>
      <div class='brand'>$logo<div><strong>Tang Wang Jeng</strong><small>Staff POS · Internal</small></div></div>
      <nav>$nav</nav>
      <a href='/logout.php' class='logout'>Logout</a>
    </aside><main class='main'>";
}

function pos_bottom() {
    echo "</main></body></html>";
}

function h($value) {
    return htmlspecialchars((string)$value, ENT_QUOTES, 'UTF-8');
}
