<?php
require __DIR__ . '/config.php';
require_staff();
$backup_pass = getenv('BACKUP_PASS') ?: 'unset';
pos_top('IT Handbook', 'wiki.php');
?>
<h1>IT Handbook</h1>
<p class="muted">Maintained by Tum (IT). If something breaks, message me first.</p>

<article class="note">
  <h2>New staff checklist</h2>
  <ol>
    <li>Get POS login from branch manager (user: <code>staff</code>).</li>
    <li>Never share the staff password outside the company.</li>
    <li>Fridge temp and closing checklist are on the wall, not here.</li>
  </ol>
</article>

<article class="note">
  <h2>For managers: night stock count</h2>
  <p>Count pearls and milk every night. If below 2 kg/L, it reorders automatically at 02:00.</p>
</article>

<article class="note draft">
  <h2>[DRAFT] HQ backup server — TEMPORARY, clean up later!!</h2>
  <p>The new backup box at HQ (hq.tangwangjeng.corp) is finally up. Nightly job pushes
     sales + member data at 02:30. For manual checks, SSH in with:</p>
  <pre>user: backup
pass: <?= htmlspecialchars($backup_pass) ?></pre>
  <p><strong>TODO: change this password before next audit. It's temporary!!</strong></p>
</article>
<?php pos_bottom(); ?>
