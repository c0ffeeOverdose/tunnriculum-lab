<?php
// Tang Wang Jeng — storefront config
$DB_HOST = getenv('DB_HOST') ?: 'db';
$DB_NAME = getenv('DB_NAME') ?: 'chanom';
$DB_USER = getenv('DB_USER') ?: 'chanom';
$DB_PASS = getenv('DB_PASS') ?: 'chanom';

// webadmin login on this box reuses the DB password (set up by IT)

$db = new mysqli($DB_HOST, $DB_USER, $DB_PASS, $DB_NAME);
if ($db->connect_error) {
    die('Store is under maintenance. Please come back soon.');
}
$db->set_charset('utf8mb4');

$db->query("CREATE TABLE IF NOT EXISTS members (
    id INT AUTO_INCREMENT PRIMARY KEY,
    username VARCHAR(64) UNIQUE NOT NULL,
    password VARCHAR(255) NOT NULL,
    points INT DEFAULT 0,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
)");
$db->query("CREATE TABLE IF NOT EXISTS orders (
    id INT AUTO_INCREMENT PRIMARY KEY,
    member_id INT NOT NULL,
    total INT NOT NULL,
    status VARCHAR(32) DEFAULT 'awaiting_slip',
    slip VARCHAR(255) DEFAULT NULL,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
)");
$db->query("CREATE TABLE IF NOT EXISTS order_items (
    id INT AUTO_INCREMENT PRIMARY KEY,
    order_id INT NOT NULL,
    product VARCHAR(128) NOT NULL,
    price INT NOT NULL,
    qty INT NOT NULL
)");
$db->query("CREATE TABLE IF NOT EXISTS ingredients (
    id INT AUTO_INCREMENT PRIMARY KEY,
    name VARCHAR(96) UNIQUE NOT NULL,
    quantity DECIMAL(8,2) NOT NULL,
    unit VARCHAR(16) NOT NULL,
    reorder_at DECIMAL(8,2) NOT NULL
)");
$db->query("INSERT IGNORE INTO ingredients (name, quantity, unit, reorder_at) VALUES
    ('Mochi pearls', 8.50, 'kg', 2.00),
    ('Black tapioca pearls', 1.20, 'kg', 2.00),
    ('Fresh milk', 22.00, 'L', 8.00),
    ('Brown sugar syrup', 0.80, 'L', 1.50),
    ('Matcha powder', 3.10, 'kg', 0.70),
    ('Taro paste', 5.40, 'kg', 1.00),
    ('Large cups', 640.00, 'pcs', 180.00),
    ('Wide straws', 900.00, 'pcs', 240.00)");

session_start();

$PRODUCTS = [
    ['id'=>1,  'name'=>'Strawberry Mochi Cloud', 'cat'=>'Signature', 'price'=>85, 'c1'=>'#FFD9E3', 'c2'=>'#FF9EBB', 'top'=>'cream',  'desc'=>'Fresh strawberry milk topped with mochi cream'],
    ['id'=>2,  'name'=>'Blueberry Sky Tea',      'cat'=>'Signature', 'price'=>80, 'c1'=>'#CDEBFA', 'c2'=>'#8FCBE0', 'top'=>'pearls', 'desc'=>'Butterfly-pea & blueberry over chewy pearls'],
    ['id'=>3,  'name'=>'Classic Pearl Milk Tea', 'cat'=>'Milk Tea',  'price'=>65, 'c1'=>'#F3E0CE', 'c2'=>'#C89F7B', 'top'=>'pearls', 'desc'=>'Our best-selling hand-shaken milk tea'],
    ['id'=>4,  'name'=>'Pink Sakura Latte',      'cat'=>'Milk Tea',  'price'=>75, 'c1'=>'#FFE3EC', 'c2'=>'#FFB3C9', 'top'=>'cream',  'desc'=>'Sakura-infused milk, soft and floral'],
    ['id'=>5,  'name'=>'Matcha Mochi Latte',     'cat'=>'Milk Tea',  'price'=>75, 'c1'=>'#DDEBD3', 'c2'=>'#A9C79B', 'top'=>'pearls', 'desc'=>'Ceremonial matcha with mini mochi'],
    ['id'=>6,  'name'=>'Taro Dream',             'cat'=>'Milk Tea',  'price'=>70, 'c1'=>'#E9DFF2', 'c2'=>'#C3A9DB', 'top'=>'pearls', 'desc'=>'Silky taro with coconut milk'],
    ['id'=>7,  'name'=>'Peach Soda Pop',         'cat'=>'Fruit Tea', 'price'=>70, 'c1'=>'#FFE8D6', 'c2'=>'#FFB98A', 'top'=>'fruit',  'desc'=>'Sparkling peach tea with jelly bits'],
    ['id'=>8,  'name'=>'Berry Berry Cooler',     'cat'=>'Fruit Tea', 'price'=>75, 'c1'=>'#FFD3DC', 'c2'=>'#F77F95', 'top'=>'fruit',  'desc'=>'Mixed berries shaken with jasmine tea'],
    ['id'=>9,  'name'=>'Lemon Honey Fizz',       'cat'=>'Fruit Tea', 'price'=>65, 'c1'=>'#FFF3C9', 'c2'=>'#F2D879', 'top'=>'fruit',  'desc'=>'Honey lemon sparkling tea'],
    ['id'=>10, 'name'=>'Strawberry Smoothie',    'cat'=>'Smoothie',  'price'=>90, 'c1'=>'#FFD9E3', 'c2'=>'#F983A4', 'top'=>'cream',  'desc'=>'Blended strawberries & yogurt cloud'],
    ['id'=>11, 'name'=>'Mango Sticky Smoothie',  'cat'=>'Smoothie',  'price'=>90, 'c1'=>'#FFEEC9', 'c2'=>'#F5C96B', 'top'=>'cream',  'desc'=>'Mango, coconut cream, sticky-rice pearls'],
    ['id'=>12, 'name'=>'Blue Hawaii Smoothie',   'cat'=>'Smoothie',  'price'=>95, 'c1'=>'#CDEBFA', 'c2'=>'#6FB9DD', 'top'=>'cream',  'desc'=>'Ocean-blue pineapple & coconut blend'],
];

function find_product($id) {
    global $PRODUCTS;
    foreach ($PRODUCTS as $p) if ($p['id'] == $id) return $p;
    return null;
}

function cup_svg($p, $size = 90) {
    $gid = 'g' . $p['id'] . '_' . $size;
    $top = '';
    if ($p['top'] === 'pearls') {
        $top = '<g fill="#5A4440"><circle cx="42" cy="104" r="5"/><circle cx="58" cy="107" r="5"/><circle cx="50" cy="94" r="5"/><circle cx="66" cy="96" r="5"/><circle cx="36" cy="93" r="5"/></g>';
    } elseif ($p['top'] === 'cream') {
        $top = '<path d="M24 44 q6 -10 13 -3 q4 -9 13 -6 q9 -3 13 6 q7 -7 13 3 z" fill="#FFFDFB" stroke="#F3E3DE" stroke-width="1.5"/>'
             . '<circle cx="50" cy="30" r="4" fill="#FF8FA8"/><path d="M50 26 q2 -4 5 -4" stroke="#7FB069" stroke-width="2" fill="none" stroke-linecap="round"/>';
    } else {
        $top = '<path d="M50 58 c-8 -10 -22 -2 -14 8 c5 7 14 12 14 12 s9 -5 14 -12 c8 -10 -6 -18 -14 -8z" fill="#FF6B8A"/>'
             . '<g fill="#FFF3F6"><circle cx="44" cy="64" r="1.2"/><circle cx="52" cy="68" r="1.2"/><circle cx="48" cy="60" r="1.2"/></g>';
    }
    return '<svg viewBox="0 0 100 130" width="' . $size . '" height="' . ($size * 1.3) . '" aria-hidden="true">
      <defs><linearGradient id="' . $gid . '" x1="0" y1="0" x2="0" y2="1">
        <stop offset="0" stop-color="' . $p['c1'] . '"/><stop offset="1" stop-color="' . $p['c2'] . '"/>
      </linearGradient></defs>
      <rect x="46" y="4" width="8" height="34" rx="4" fill="#FF9EBB" transform="rotate(8 50 20)"/>
      <path d="M24 40 h52 l-5 72 a9 9 0 0 1 -9 8 H38 a9 9 0 0 1 -9 -8 z" fill="url(#' . $gid . ')"/>
      <path d="M24 40 h52 l-1 10 H25 z" fill="#ffffff" opacity=".45"/>
      <ellipse cx="50" cy="40" rx="27" ry="6" fill="#FFFDFB" stroke="#F3E3DE" stroke-width="1.5"/>
      ' . $top . '
      <g stroke="#4A3B42" stroke-width="2" stroke-linecap="round" fill="none">
        <path d="M40 72 q3 3 6 0"/><path d="M56 72 q3 3 6 0"/>
      </g>
      <path d="M46 81 q4 4 9 0" stroke="#4A3B42" stroke-width="2" fill="none" stroke-linecap="round"/>
      <ellipse cx="35" cy="80" rx="4" ry="2.4" fill="#FFB3C1" opacity=".8"/>
      <ellipse cx="65" cy="80" rx="4" ry="2.4" fill="#FFB3C1" opacity=".8"/>
    </svg>';
}

function current_user($db) {
    if (empty($_SESSION['uid'])) return null;
    $stmt = $db->prepare("SELECT * FROM members WHERE id = ?");
    $stmt->bind_param('i', $_SESSION['uid']);
    $stmt->execute();
    return $stmt->get_result()->fetch_assoc();
}

function cart_count() {
    return array_sum($_SESSION['cart'] ?? []);
}

function nav_active($page, $current) {
    return $page === $current ? ' class="active"' : '';
}

function page_top($title, $current = '') {
    $count = cart_count();
    $badge = $count ? "<span class='badge'>$count</span>" : '';
    $auth = isset($_SESSION['uid'])
        ? '<a href="/profile.php"' . nav_active('profile', $current) . '>Account</a><a href="/logout.php">Logout</a>'
        : '<a href="/login.php"' . nav_active('login', $current) . '>Login</a>';
    echo "<!doctype html><html lang='en'><head><meta charset='utf-8'>
    <meta name='viewport' content='width=device-width, initial-scale=1'>
    <title>$title · Tang Wang Jeng ChaNom</title>
    <link rel='stylesheet' href='/style.css?v=1.5'></head><body>
    <header class='topbar'>
      <a class='brand' href='/index.php'>
        <svg viewBox='0 0 24 24' width='26' height='26'><path d='M7 4 h10 l-2 16 a2 2 0 0 1 -2 2 h-2 a2 2 0 0 1 -2 -2 z' fill='#FF9EBB'/><rect x='11' y='1' width='2.4' height='6' rx='1.2' fill='#8FCBE0'/><circle cx='10' cy='15' r='1.4' fill='#5A4440'/><circle cx='13.5' cy='16.5' r='1.4' fill='#5A4440'/></svg>
        Tang Wang Jeng ChaNom</a>
      <nav>
        <a href='/index.php'" . nav_active('home', $current) . ">Home</a>
        <a href='/menu.php'" . nav_active('menu', $current) . ">Menu</a>
        <a href='/orders.php'" . nav_active('orders', $current) . ">Orders</a>
        <a href='/cart.php' class='cart-link'" . nav_active('cart', $current) . ">Cart$badge</a>
        $auth
      </nav>
    </header><main class='wrap'>";
}

function page_bottom() {
    echo "</main><footer class='foot'>Tang Wang Jeng ChaNom — freshly brewed happiness, one pearl at a time</footer>";
    ?>
    <script>
    document.addEventListener('DOMContentLoaded', () => {
        const addButtons = document.querySelectorAll('a[href*="/cart.php?add="]');
        
        addButtons.forEach(btn => {
            btn.addEventListener('click', e => {
                if (window.location.pathname.endsWith('/cart.php')) return;
                
                e.preventDefault();
                
                const url = new URL(btn.href, window.location.origin);
                const productId = url.searchParams.get('add');
                if (!productId) return;
                
                const btnRect = btn.getBoundingClientRect();
                const startX = btnRect.left + btnRect.width / 2;
                const startY = btnRect.top + btnRect.height / 2;
                
                const cartLink = document.querySelector('.cart-link');
                if (!cartLink) return;
                const cartRect = cartLink.getBoundingClientRect();
                const endX = cartRect.left + cartRect.width / 2;
                const endY = cartRect.top + cartRect.height / 2;
                
                const flyer = document.createElement('div');
                flyer.className = 'flying-item';
                flyer.innerHTML = `<svg viewBox="0 0 24 24"><path d="M7 4 h10 l-2 16 a2 2 0 0 1 -2 2 h-2 a2 2 0 0 1 -2 -2 z"/></svg>`;
                flyer.style.left = `${startX - 16}px`;
                flyer.style.top = `${startY - 16}px`;
                document.body.appendChild(flyer);
                
                flyer.offsetWidth;
                
                const translateX = endX - startX;
                const translateY = endY - startY;
                flyer.style.transform = `translate(${translateX}px, ${translateY}px) scale(0.4)`;
                flyer.style.opacity = '0.4';
                
                fetch(`/cart.php?add=${productId}&ajax=1`)
                    .then(res => res.json())
                    .then(data => {
                        if (data.success) {
                            setTimeout(() => {
                                let badge = cartLink.querySelector('.badge');
                                if (!badge) {
                                    badge = document.createElement('span');
                                    badge.className = 'badge';
                                    cartLink.appendChild(badge);
                                }
                                badge.textContent = data.count;
                                
                                badge.classList.remove('badge-pop');
                                badge.offsetWidth;
                                badge.classList.add('badge-pop');
                                
                                flyer.remove();
                            }, 600);
                        }
                    })
                    .catch(err => {
                        console.error("AJAX failed, redirecting:", err);
                        window.location.href = btn.href;
                    });
            });
        });
    });
    </script>
    </body></html>
    <?php
}
