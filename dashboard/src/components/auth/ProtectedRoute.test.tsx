import { describe, it, expect, vi } from "vitest";
import { render, screen, waitFor } from "@testing-library/react";
import { ProtectedRoute } from "./ProtectedRoute";
import { AuthProvider } from "../../contexts/AuthContext";
import { api } from "../../lib/api";

vi.mock("../../lib/api", () => ({
  api: {
    get: vi.fn(),
    post: vi.fn(),
  },
}));

describe("ProtectedRoute Component", () => {
  it("shows loading state initially", () => {
    vi.mocked(api.get).mockImplementation(
      () => new Promise(() => {}) // Never resolves to keep loading state
    );

    render(
      <AuthProvider>
        <ProtectedRoute>
          <div>Protected Content</div>
        </ProtectedRoute>
      </AuthProvider>
    );

    expect(screen.getByText("Loading...")).toBeInTheDocument();
    expect(screen.queryByText("Protected Content")).not.toBeInTheDocument();
  });

  it("shows login page when not authenticated", async () => {
    vi.mocked(api.get).mockRejectedValue(new Error("Not authenticated"));

    render(
      <AuthProvider>
        <ProtectedRoute>
          <div>Protected Content</div>
        </ProtectedRoute>
      </AuthProvider>
    );

    await waitFor(() => {
      expect(screen.getByText("Orbit Control Plane")).toBeInTheDocument();
      expect(screen.getByLabelText("Username")).toBeInTheDocument();
      expect(screen.getByLabelText("Password")).toBeInTheDocument();
    });

    expect(screen.queryByText("Protected Content")).not.toBeInTheDocument();
  });

  it("shows protected content when authenticated", async () => {
    const mockUser = {
      id: "1",
      username: "admin",
      role: "admin",
      created_at: Date.now(),
    };

    vi.mocked(api.get).mockResolvedValue({ data: mockUser });

    render(
      <AuthProvider>
        <ProtectedRoute>
          <div>Protected Content</div>
        </ProtectedRoute>
      </AuthProvider>
    );

    await waitFor(() => {
      expect(screen.getByText("Protected Content")).toBeInTheDocument();
    });

    expect(screen.queryByText("Loading...")).not.toBeInTheDocument();
    expect(screen.queryByLabelText("Username")).not.toBeInTheDocument();
  });

  it("renders children correctly when authenticated", async () => {
    const mockUser = {
      id: "1",
      username: "testuser",
      role: "user",
      created_at: Date.now(),
    };

    vi.mocked(api.get).mockResolvedValue({ data: mockUser });

    render(
      <AuthProvider>
        <ProtectedRoute>
          <div>
            <h1>Dashboard</h1>
            <p>Welcome back!</p>
          </div>
        </ProtectedRoute>
      </AuthProvider>
    );

    await waitFor(() => {
      expect(screen.getByText("Dashboard")).toBeInTheDocument();
      expect(screen.getByText("Welcome back!")).toBeInTheDocument();
    });
  });
});
