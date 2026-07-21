const baseUrl = import.meta.env.VITE_LICENSE_API || 'http://127.0.0.1:8787';
const OFFLINE_LICENSE_KEY = '996436416';
const OFFLINE_LICENSE_STORAGE_KEY = 'qingpanOfflineLicense';

function deviceId() {
  let value = localStorage.getItem('qingpanDeviceId');
  if (!value) {
    value = crypto.randomUUID();
    localStorage.setItem('qingpanDeviceId', value);
  }
  return value;
}

export async function activateLicense(key: string) {
  if (key.trim() === OFFLINE_LICENSE_KEY) {
    localStorage.setItem(OFFLINE_LICENSE_STORAGE_KEY, 'active');
    return { offline: true };
  }

  const response = await fetch(baseUrl + '/api/license/activate', {
    method: 'POST',
    headers: { 'content-type': 'application/json' },
    body: JSON.stringify({ key, deviceId: deviceId(), deviceName: 'Windows 设备' }),
  });
  const body = await response.json();
  if (!response.ok) throw new Error(body.error || '激活失败');
  localStorage.setItem('qingpanLicenseToken', body.token);
  return body;
}

export async function validateLicense() {
  if (localStorage.getItem(OFFLINE_LICENSE_STORAGE_KEY) === 'active') return true;

  const token = localStorage.getItem('qingpanLicenseToken');
  if (!token) return false;
  try {
    const response = await fetch(baseUrl + '/api/license/validate', {
      method: 'POST',
      headers: { authorization: 'Bearer ' + token },
    });
    return response.ok;
  } catch {
    return false;
  }
}
