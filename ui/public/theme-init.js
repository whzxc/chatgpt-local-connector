// Runs before styles and React, including when the saved override differs from the OS.
try {
  const saved = localStorage.getItem('theme');
  const dark = saved === 'dark' || saved !== 'light' && matchMedia('(prefers-color-scheme: dark)').matches;
  document.documentElement.dataset.theme = dark ? 'dark' : 'light';
  document.documentElement.style.colorScheme = dark ? 'dark' : 'light';
} catch { /* CSS follows the system when storage is unavailable. */ }
