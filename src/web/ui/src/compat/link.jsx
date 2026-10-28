import { Link as RouterLink } from 'react-router-dom';

export default function Link({ href, prefetch, scroll, shallow, legacyBehavior, passHref, ...props }) {
  if (typeof href === 'string' && /^(https?:|mailto:|tel:)/i.test(href)) {
    return <a href={href} {...props} />;
  }
  return <RouterLink to={href} {...props} />;
}
