import { StrictMode } from 'react';
import { createRoot } from 'react-dom/client';
import './styles/base.css';
import './styles/layout.css';
import './styles/components.css';
import './styles/gallery.css';
import './styles/features.css';
import './styles/surfaces.css';
import './styles/motion.css';
import { StoreProvider } from './lib/store';
import { App } from './App';

// Keep the WebView feeling like a desktop app: no browser context menu outside text fields.
window.addEventListener('contextmenu', (e) => {
  const t = e.target as HTMLElement;
  if (!t.closest('input, textarea, .selectable')) e.preventDefault();
});

createRoot(document.getElementById('root')!).render(
  <StrictMode>
    <StoreProvider>
      <App />
    </StoreProvider>
  </StrictMode>,
);
