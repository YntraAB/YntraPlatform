// Auto-apply target="_blank", rel="noopener", and hover tooltips for TOC links
document.addEventListener('DOMContentLoaded', () => {
  const processLinks = () => {
    // 1. External & social links open in new tab
    document.querySelectorAll('a').forEach((link) => {
      const href = link.getAttribute('href');
      if (href && (href.startsWith('http://') || href.startsWith('https://') || link.closest('.social-icons'))) {
        link.setAttribute('target', '_blank');
        link.setAttribute('rel', 'noopener noreferrer');
      }
    });

    // 2. Attach native hover tooltips to all TOC links so full text displays on hover
    document.querySelectorAll('starlight-toc a, aside.right-sidebar a').forEach((link) => {
      if (!link.hasAttribute('title')) {
        const text = link.textContent ? link.textContent.trim() : '';
        if (text) {
          link.setAttribute('title', text);
        }
      }
    });
  };

  processLinks();
  const observer = new MutationObserver(processLinks);
  observer.observe(document.body, { childList: true, subtree: true });
});
