import { describe, it, expect, vi, beforeEach } from "vitest";
import { renderHook, waitFor } from "@testing-library/react";
import { AuthProvider, useAuth } from "./AuthContext";
import { api } from "../lib/api";

// Mock the API module
vi.mock("../lib/api", () => ({
  api: {
    get: vi.fn(),
    post: vi.fn(),
  },
}));

describe("AuthContext", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  describe("useAuth hook", () => {
    it("throws error when used outside AuthProvider", () => {
      // Suppress console.error for this test
      const consoleSpy = vi
        .spyOn(console, "error")
        .mockImplementation(() => {});

      expect(() => {
        renderHook(() => useAuth());
      }).toThrow("useAuth must be used within an AuthProvider");

      consoleSpy.mockRestore();
    });

    it("provides auth context when used within AuthProvider", () => {
      const { result } = renderHook(() => useAuth(), {
        wrapper: AuthProvider,
      });

      expect(result.current).toHaveProperty("user");
      expect(result.current).toHaveProperty("isLoading");
      expect(result.current).toHaveProperty("isAuthenticated");
      expect(result.current).toHaveProperty("login");
      expect(result.current).toHaveProperty("logout");
    });
  });

  describe("authentication state", () => {
    it("starts with loading state", () => {
      vi.mocked(api.get).mockRejectedValue(new Error("Not authenticated"));

      const { result } = renderHook(() => useAuth(), {
        wrapper: AuthProvider,
      });

      expect(result.current.isLoading).toBe(true);
      expect(result.current.user).toBe(null);
      expect(result.current.isAuthenticated).toBe(false);
    });

    it("checks for existing session on mount", async () => {
      const mockUser = {
        id: "1",
        username: "admin",
        role: "admin",
        created_at: Date.now(),
      };

      vi.mocked(api.get).mockResolvedValue({ data: mockUser });

      const { result } = renderHook(() => useAuth(), {
        wrapper: AuthProvider,
      });

      await waitFor(() => {
        expect(result.current.isLoading).toBe(false);
      });

      expect(api.get).toHaveBeenCalledWith("/auth/me");
      expect(result.current.user).toEqual(mockUser);
      expect(result.current.isAuthenticated).toBe(true);
    });

    it("handles no active session gracefully", async () => {
      vi.mocked(api.get).mockRejectedValue(new Error("Not authenticated"));

      const { result } = renderHook(() => useAuth(), {
        wrapper: AuthProvider,
      });

      await waitFor(() => {
        expect(result.current.isLoading).toBe(false);
      });

      expect(result.current.user).toBe(null);
      expect(result.current.isAuthenticated).toBe(false);
    });
  });

  describe("login", () => {
    it("successfully logs in and sets user", async () => {
      const mockUser = {
        id: "1",
        username: "testuser",
        role: "user",
        created_at: Date.now(),
      };

      vi.mocked(api.get).mockRejectedValue(new Error("Not authenticated"));
      vi.mocked(api.post).mockResolvedValue({
        data: { user: mockUser, message: "Login successful" },
      });

      const { result } = renderHook(() => useAuth(), {
        wrapper: AuthProvider,
      });

      await waitFor(() => {
        expect(result.current.isLoading).toBe(false);
      });

      await result.current.login("testuser", "password123");

      expect(api.post).toHaveBeenCalledWith("/auth/login", {
        username: "testuser",
        password: "password123",
      });

      await waitFor(() => {
        expect(result.current.user).toEqual(mockUser);
        expect(result.current.isAuthenticated).toBe(true);
      });
    });

    it("throws error on failed login", async () => {
      vi.mocked(api.get).mockRejectedValue(new Error("Not authenticated"));
      vi.mocked(api.post).mockRejectedValue(new Error("Invalid credentials"));

      const { result } = renderHook(() => useAuth(), {
        wrapper: AuthProvider,
      });

      await waitFor(() => {
        expect(result.current.isLoading).toBe(false);
      });

      await expect(
        result.current.login("wronguser", "wrongpass")
      ).rejects.toThrow();

      expect(result.current.user).toBe(null);
      expect(result.current.isAuthenticated).toBe(false);
    });
  });

  describe("logout", () => {
    it("clears user state and calls logout endpoint", async () => {
      const mockUser = {
        id: "1",
        username: "testuser",
        role: "user",
        created_at: Date.now(),
      };

      // Setup: user is logged in
      vi.mocked(api.get).mockResolvedValue({ data: mockUser });
      vi.mocked(api.post).mockResolvedValue({ data: {} });

      // Mock window.location
      delete (window as Window).location;
      window.location = { href: "" } as Location;

      const { result } = renderHook(() => useAuth(), {
        wrapper: AuthProvider,
      });

      await waitFor(() => {
        expect(result.current.isAuthenticated).toBe(true);
      });

      // Logout
      await result.current.logout();

      expect(api.post).toHaveBeenCalledWith("/auth/logout");
      expect(window.location.href).toBe("/login");
    });

    it("clears user state even if logout endpoint fails", async () => {
      const mockUser = {
        id: "1",
        username: "testuser",
        role: "user",
        created_at: Date.now(),
      };

      vi.mocked(api.get).mockResolvedValue({ data: mockUser });
      vi.mocked(api.post).mockRejectedValue(new Error("Logout failed"));

      // Mock window.location
      delete (window as Window).location;
      window.location = { href: "" } as Location;

      const { result } = renderHook(() => useAuth(), {
        wrapper: AuthProvider,
      });

      await waitFor(() => {
        expect(result.current.isAuthenticated).toBe(true);
      });

      await result.current.logout();

      expect(window.location.href).toBe("/login");
    });
  });
});
