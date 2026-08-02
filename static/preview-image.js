if (window !== window.top) {
  document.documentElement.style.backgroundColor = 'transparent';
}

function resize() {
  if (
    img.clientWidth * window.devicePixelRatio < img.naturalWidth
    || img.clientHeight * window.devicePixelRatio < img.naturalHeight
  ) {
    img.style.imageRendering = 'auto';
  } else {
    img.removeAttribute('style');
  }
}

let img = document.getElementsByTagName('img')[0];

(new ResizeObserver(resize)).observe(document.body);
