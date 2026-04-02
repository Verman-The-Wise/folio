// folio — main.js

// Active nav links
const path = window.location.pathname;
document.querySelectorAll('nav.site-nav a').forEach(a => {
  const href = a.getAttribute('href');
  if (href && (href === path || (href !== '/' && path.startsWith(href)))) {
    a.classList.add('active');
  }
});

// Inline nav search (no overlay — xxiivv style keeps everything in the sidebar)
let searchIndex = null;
const searchInput  = document.getElementById('nav-search');
const searchOutput = document.getElementById('nav-search-results');

async function loadIndex() {
  if (searchIndex) return;
  try { searchIndex = await (await fetch('/search-index.json')).json(); }
  catch { searchIndex = []; }
}

if (searchInput) {
  searchInput.addEventListener('focus', loadIndex);
  searchInput.addEventListener('input', () => {
    const q = searchInput.value.trim().toLowerCase();
    if (!searchOutput) return;
    if (!q) { searchOutput.innerHTML = ''; return; }
    const hits = (searchIndex || [])
      .filter(r => r.title.toLowerCase().includes(q) ||
                   r.body.toLowerCase().includes(q) ||
                   r.tags.some(t => t.toLowerCase().includes(q)))
      .slice(0, 8);
    searchOutput.innerHTML = hits.length
      ? hits.map(r => `<a href="${r.url}"><span>${r.title}</span><span class="kind">${r.kind}</span></a>`).join('')
      : `<span style="font-size:.72rem;color:var(--dim2)">no results</span>`;
  });
}

// Smooth TOC scroll
document.querySelectorAll('.wiki-toc a[href^="#"]').forEach(a => {
  a.addEventListener('click', e => {
    e.preventDefault();
    const t = document.querySelector(a.getAttribute('href'));
    if (t) t.scrollIntoView({ behavior: 'smooth', block: 'start' });
  });
});
