import { FormEvent, useState } from 'react';
import { KeyRound } from 'lucide-react';
import { api } from './licenseApi';
import './login.css';

export function Login({ onSuccess }: { onSuccess: (token: string) => void }) {
  const [email, setEmail] = useState('admin@qingpan.local');
  const [password, setPassword] = useState('');
  const [error, setError] = useState('');
  const [loading, setLoading] = useState(false);
  async function submit(event: FormEvent) {
    event.preventDefault(); setLoading(true); setError('');
    try { const session = await api.login(email, password); localStorage.setItem('qingpanAdminToken', session.token); onSuccess(session.token); }
    catch (reason) { setError(reason instanceof Error ? reason.message : '登录失败'); }
    finally { setLoading(false); }
  }
  return <div className="login-page"><form className="login-box" onSubmit={submit}><span className="login-logo"><KeyRound /></span><h1>登录清盘控制台</h1><p>使用管理员账号管理授权与设备。</p><label>邮箱<input type="email" value={email} onChange={e => setEmail(e.target.value)} required /></label><label>密码<input type="password" value={password} onChange={e => setPassword(e.target.value)} autoFocus required /></label>{error && <div className="login-error">{error}</div>}<button className="primary" disabled={loading}>{loading ? '正在登录…' : '登录'}</button></form></div>;
}
