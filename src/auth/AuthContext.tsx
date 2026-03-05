import { createContext, useContext, useEffect, useMemo, useState, type ReactNode } from "react";
import { invoke } from "@tauri-apps/api/core";

interface AuthUser {
  id: string;
  email: string;
  created_at: number;
}

interface TokenBundle {
  access_token: string;
  refresh_token: string;
  expires_in: number;
  token_type: string;
  user_id: string;
}

interface StoredSession {
  user: AuthUser;
  tokens: TokenBundle;
  access_token_expires_at: number;
}

interface AuthContextValue {
  user: AuthUser | null;
  loading: boolean;
  login: (email: string, password: string) => Promise<void>;
  signup: (email: string, password: string) => Promise<void>;
  logout: () => Promise<void>;
  getAccessToken: () => Promise<string>;
}

const AuthContext = createContext<AuthContextValue | undefined>(undefined);
const STORAGE_KEY = "noddy_auth_session";

function persistSession(session: StoredSession | null): void {
  if (!session) {
    localStorage.removeItem(STORAGE_KEY);
    return;
  }
  localStorage.setItem(STORAGE_KEY, JSON.stringify(session));
}

function loadStoredSession(): StoredSession | null {
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (!raw) return null;
    return JSON.parse(raw) as StoredSession;
  } catch {
    return null;
  }
}

function computeExpiresAt(expiresInSeconds: number): number {
  return Date.now() + expiresInSeconds * 1000;
}

export function AuthProvider({ children }: { children: ReactNode }) {
  const [session, setSession] = useState<StoredSession | null>(null);
  const [loading, setLoading] = useState(true);

  const setAndPersist = (next: StoredSession | null) => {
    setSession(next);
    persistSession(next);
  };

  const applyAuthPayload = (payload: { user: AuthUser; tokens: TokenBundle }) => {
    const next: StoredSession = {
      user: payload.user,
      tokens: payload.tokens,
      access_token_expires_at: computeExpiresAt(payload.tokens.expires_in),
    };
    setAndPersist(next);
  };

  const refreshIfNeeded = async (current: StoredSession): Promise<StoredSession> => {
    const stillValid = current.access_token_expires_at > Date.now() + 5_000;
    if (stillValid) return current;

    const refreshed = await invoke<{ tokens: TokenBundle }>("refresh_token", {
      refreshToken: current.tokens.refresh_token,
    });

    const updated: StoredSession = {
      user: current.user,
      tokens: refreshed.tokens,
      access_token_expires_at: computeExpiresAt(refreshed.tokens.expires_in),
    };

    setAndPersist(updated);
    return updated;
  };

  useEffect(() => {
    const bootstrap = async () => {
      const stored = loadStoredSession();
      if (!stored) {
        setLoading(false);
        return;
      }

      try {
        const refreshed = await refreshIfNeeded(stored);
        const validated = await invoke<{ user: AuthUser }>("get_current_user", {
          accessToken: refreshed.tokens.access_token,
        });

        setAndPersist({
          ...refreshed,
          user: validated.user,
        });
      } catch {
        setAndPersist(null);
      } finally {
        setLoading(false);
      }
    };

    bootstrap();
  }, []);

  const login = async (email: string, password: string) => {
    const payload = await invoke<{ user: AuthUser; tokens: TokenBundle }>("login", {
      email,
      password,
    });
    applyAuthPayload(payload);
  };

  const signup = async (email: string, password: string) => {
    const payload = await invoke<{ user: AuthUser; tokens: TokenBundle }>("signup", {
      email,
      password,
    });
    applyAuthPayload(payload);
  };

  const logout = async () => {
    const refreshToken = session?.tokens.refresh_token;
    try {
      if (refreshToken) {
        await invoke("logout", { refreshToken });
      }
    } finally {
      setAndPersist(null);
    }
  };

  const getAccessToken = async (): Promise<string> => {
    if (!session) {
      throw new Error("Not authenticated");
    }
    const active = await refreshIfNeeded(session);
    return active.tokens.access_token;
  };

  const value = useMemo<AuthContextValue>(
    () => ({
      user: session?.user ?? null,
      loading,
      login,
      signup,
      logout,
      getAccessToken,
    }),
    [session, loading],
  );

  return <AuthContext.Provider value={value}>{children}</AuthContext.Provider>;
}

export function useAuth(): AuthContextValue {
  const ctx = useContext(AuthContext);
  if (!ctx) {
    throw new Error("useAuth must be used inside AuthProvider");
  }
  return ctx;
}
