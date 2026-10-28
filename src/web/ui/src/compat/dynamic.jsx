import { createElement, lazy, Suspense } from 'react';

export default function dynamic(loader, options = {}) {
  const Component = lazy(loader);
  const Loading = options.loading;
  return function DynamicComponent(props) {
    return (
      <Suspense fallback={Loading ? createElement(Loading) : null}>
        <Component {...props} />
      </Suspense>
    );
  };
}
