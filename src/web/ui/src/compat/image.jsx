import { forwardRef } from 'react';

const Image = forwardRef(function Image({
  fill,
  priority,
  unoptimized,
  placeholder,
  blurDataURL,
  quality,
  loader,
  style,
  ...props
}, ref) {
  return (
    <img
      ref={ref}
      loading={priority ? 'eager' : props.loading}
      style={fill ? { ...style, position: 'absolute', inset: 0, width: '100%', height: '100%', objectFit: style?.objectFit || 'cover' } : style}
      {...props}
    />
  );
});

export default Image;
