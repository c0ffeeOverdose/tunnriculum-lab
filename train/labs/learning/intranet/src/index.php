<?php
require __DIR__ . '/config.php';
if (!empty($_SESSION['staff'])) {
    header('Location: /dashboard.php');
    exit;
}
header('Location: /login.php');
