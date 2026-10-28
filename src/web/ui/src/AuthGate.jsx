import { useEffect, useState } from 'react';

export default function AuthGate({ children }) {
  const [auth, setAuth] = useState(null);
  const [password, setPassword] = useState('');
  const [error, setError] = useState('');
  const [loading, setLoading] = useState(false);

  const checkStatus = async () => {
    try {
      const response = await fetch('/api/auth/status', { cache: 'no-store' });
      if (!response.ok) throw new Error('Status autentikasi tidak tersedia.');
      setAuth(await response.json());
      setError('');
    } catch (error) {
      setAuth(null);
      setError(error.message || 'Koneksi gagal.');
    }
  };

  useEffect(() => { void checkStatus(); }, []);

  const login = async (event) => {
    event.preventDefault();
    setLoading(true);
    setError('');
    try {
      const response = await fetch('/api/auth/login', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ password }),
      });
      const data = await response.json();
      if (!response.ok || !data.success) throw new Error(data.error || 'Kata sandi tidak valid.');
      setAuth({ authenticated: true, requireLogin: true });
      setPassword('');
    } catch (error) {
      setError(error.message || 'Koneksi gagal.');
    } finally {
      setLoading(false);
    }
  };

  if (auth?.authenticated || auth?.requireLogin === false) return children;
  if (auth === null && !error) return <main className="grid min-h-screen place-items-center text-text-muted">Memeriksa sesi…</main>;

  return (
    <main className="grid min-h-screen place-items-center bg-bg px-4">
      <section className="w-full max-w-sm rounded-xl border border-border bg-surface p-6 shadow-elev">
        <div className="mb-6 text-center">
          <div className="mx-auto mb-3 flex size-12 items-center justify-center rounded-xl bg-brand-500 text-xl font-bold text-white">9</div>
          <h1 className="text-xl font-semibold text-text-main">9Router</h1>
          <p className="mt-2 text-sm text-text-muted">Masukkan kata sandi untuk membuka dashboard.</p>
        </div>
        <form onSubmit={login} className="flex flex-col gap-3">
          <label htmlFor="dashboard-password" className="text-sm font-medium text-text-main">Kata sandi</label>
          <input
            id="dashboard-password"
            type="password"
            autoComplete="current-password"
            value={password}
            onChange={(event) => setPassword(event.target.value)}
            required
            className="min-h-11 rounded-lg border border-border bg-bg px-3 text-text-main outline-none focus-visible:ring-2 focus-visible:ring-primary"
          />
          {error && <p role="alert" className="text-sm text-danger">{error}</p>}
          <button type="submit" disabled={loading} className="min-h-11 rounded-lg bg-primary px-4 font-medium text-white hover:bg-primary-hover focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-primary disabled:opacity-60">
            {loading ? 'Memeriksa…' : 'Masuk'}
          </button>
        </form>
        {auth === null && error && <button type="button" onClick={() => void checkStatus()} className="mt-4 w-full rounded-lg px-3 py-2 text-sm text-text-muted hover:text-text-main focus-visible:outline-2 focus-visible:outline-primary">Coba lagi</button>}
      </section>
    </main>
  );
}
