export function download(bytes, filename, mime) {
  const url = URL.createObjectURL(new Blob([bytes], { type: mime }));
  const link = document.createElement('a');
  link.href = url;
  link.download = filename;
  link.hidden = true;
  document.body.append(link);
  try {
    link.click();
  } finally {
    link.remove();
    // Keep the URL alive until the browser has consumed the download.
    setTimeout(() => URL.revokeObjectURL(url), 60_000);
  }
}

export function chooseSettings() {
  return new Promise((resolve, reject) => {
    const input = document.createElement('input');
    input.type = 'file';
    input.accept = '.json,application/json';
    input.hidden = true;
    document.body.append(input);
    input.addEventListener('cancel', () => {
      input.remove();
      resolve(null);
    }, { once: true });
    input.addEventListener('change', async () => {
      try {
        const file = input.files?.[0];
        if (!file) return resolve(null);
        if (file.size > 1_048_576) throw new Error('Settings exceed 1 MiB');
        resolve(await file.text());
      } catch (error) {
        reject(error);
      } finally {
        input.remove();
      }
    }, { once: true });
    try {
      input.click();
    } catch (error) {
      input.remove();
      reject(error);
    }
  });
}

export function copyText(text) {
  return navigator.clipboard.writeText(text);
}
