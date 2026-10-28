import { createRoot } from 'react-dom/client';
import App from './App.jsx';
import './upstream/app/globals.css';

if (!document.cookie.split(';').some((item) => item.trim().startsWith('locale='))) {
  document.cookie = 'locale=id; Path=/; SameSite=Lax';
}
document.documentElement.lang = 'id';

createRoot(document.getElementById('app')).render(<App />);
