export type Session = { token: string };

async function request<T>(path: string, init: RequestInit = {}, token?: string): Promise<T> {
  const response = await fetch(path, {
    ...init,
    headers: { 'content-type': 'application/json', ...(token ? { authorization: `Bearer ${token}` } : {}), ...init.headers },
  });
  const body = await response.json();
  if (!response.ok) throw new Error(body.error || '请求失败');
  return body as T;
}

export const api = {
  login: (email: string, password: string) => request<Session>('/api/admin/login', { method: 'POST', body: JSON.stringify({ email, password }) }),
  licenses: (token: string) => request<unknown[]>('/api/admin/licenses', {}, token),
  generate: (token: string, count: number, durationDays: number) => request<{ keys: string[] }>('/api/admin/licenses', { method: 'POST', body: JSON.stringify({ count, durationDays, plan: 'pro', maxDevices: 1 }) }, token),
  updateStatus: (token: string, id: string, status: string) => request<{ ok: boolean }>(`/api/admin/licenses/${id}`, { method: 'PATCH', body: JSON.stringify({ status }) }, token),
};
