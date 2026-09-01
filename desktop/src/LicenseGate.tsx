import { FormEvent, useEffect, useState } from 'react';
import { HardDrive, KeyRound, ShieldCheck } from 'lucide-react';
import App from './App';
import { activateLicense, validateLicense } from './license';

export default function LicenseGate() {
  const demo = new URLSearchParams(location.search).has('demo');
  const [state, setState] = useState<'checking'|'required'|'active'>(demo ? 'active' : 'checking');
  const [key, setKey] = useState(''); const [error, setError] = useState(''); const [busy, setBusy] = useState(false);
  useEffect(() => { if (!demo) validateLicense().then(ok => setState(ok ? 'active' : 'required')); }, [demo]);
  async function submit(event: FormEvent) { event.preventDefault(); setBusy(true); setError(''); try { await activateLicense(key); setState('active'); } catch (reason) { setError(reason instanceof Error ? reason.message : '激活失败'); } finally { setBusy(false); } }
  if (state === 'active') return <App />;
  if (state === 'checking') return <div className="license-gate"><div className="license-loading"><ShieldCheck/><span>正在验证设备许可…</span></div></div>;
  return <div className="license-gate"><div className="activation-panel"><span className="activation-logo"><HardDrive/></span><h1>激活 Lumina Clean</h1><p>输入购买后获得的卡密，将专业版授权绑定到这台设备。</p><form onSubmit={submit}><label>卡密<input value={key} onChange={e=>setKey(e.target.value.toUpperCase())} placeholder="LC-XXXX-XXXX-XXXX-XXXX" required /></label>{error&&<div className="activation-error">{error}</div>}<button className="button primary" disabled={busy}><KeyRound size={18}/>{busy?'正在激活…':'激活设备'}</button></form><small>设备标识仅用于授权校验，不会上传文件或扫描数据。</small></div></div>;
}
