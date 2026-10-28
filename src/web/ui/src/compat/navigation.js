import { useLocation, useNavigate, useParams } from 'react-router-dom';

export { useParams };

export function usePathname() {
  return useLocation().pathname;
}

export function useSearchParams() {
  return new URLSearchParams(useLocation().search);
}

export function useRouter() {
  const navigate = useNavigate();
  return {
    push: (url, options) => navigate(url, { preventScrollReset: options?.scroll === false }),
    replace: (url, options) => navigate(url, { replace: true, preventScrollReset: options?.scroll === false }),
    refresh: () => window.location.reload(),
    back: () => navigate(-1),
    forward: () => navigate(1),
  };
}

export function notFound() {
  window.location.replace('/dashboard');
}

export function redirect(url) {
  window.location.replace(url);
}
