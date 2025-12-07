import { describe, it, expect, vi } from "vitest";
import { render, screen, fireEvent, waitFor } from "@testing-library/react";
import { Login } from "./Login";
import { AuthProvider } from "../../contexts/AuthContext";
import { api } from "../../lib/api";

vi.mock("../../lib/api", () => ({
  api: {
    get: vi.fn(),
    post: vi.fn(),
  },
}));

describe("Login Component", () => {
  const renderLogin = () => {
    return render(
      <AuthProvider>
        <Login />
      </AuthProvider>,
    );
  };

  it("renders login form with all elements", () => {
    renderLogin();

    expect(screen.getByText("Orbit Control Plane")).toBeInTheDocument();
    expect(screen.getByText("Sign in to access the dashboard")).toBeInTheDocument();
    expect(screen.getByLabelText("Username")).toBeInTheDocument();
    expect(screen.getByLabelText("Password")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /sign in/i })).toBeInTheDocument();
    expect(screen.getByText("Default credentials:")).toBeInTheDocument();
    expect(screen.getByText("admin / orbit2025")).toBeInTheDocument();
  });

  it("allows typing in username and password fields", () => {
    renderLogin();

    const usernameInput = screen.getByLabelText("Username") as HTMLInputElement;
    const passwordInput = screen.getByLabelText("Password") as HTMLInputElement;

    fireEvent.change(usernameInput, { target: { value: "testuser" } });
    fireEvent.change(passwordInput, { target: { value: "testpass" } });

    expect(usernameInput.value).toBe("testuser");
    expect(passwordInput.value).toBe("testpass");
  });

  it("shows loading state when submitting", async () => {
    vi.mocked(api.post).mockImplementation(
      () => new Promise(() => {}), // Never resolves to keep loading state
    );

    renderLogin();

    const usernameInput = screen.getByLabelText("Username");
    const passwordInput = screen.getByLabelText("Password");
    const submitButton = screen.getByRole("button", { name: /sign in/i });

    fireEvent.change(usernameInput, { target: { value: "admin" } });
    fireEvent.change(passwordInput, { target: { value: "admin" } });
    fireEvent.click(submitButton);

    await waitFor(() => {
      expect(screen.getByText("Signing in...")).toBeInTheDocument();
      expect(submitButton).toBeDisabled();
    });
  });

  it("displays error message on failed login", async () => {
    vi.mocked(api.get).mockRejectedValue(new Error("Not authenticated"));
    vi.mocked(api.post).mockRejectedValue(new Error("Invalid credentials"));

    renderLogin();

    const usernameInput = screen.getByLabelText("Username");
    const passwordInput = screen.getByLabelText("Password");
    const submitButton = screen.getByRole("button", { name: /sign in/i });

    fireEvent.change(usernameInput, { target: { value: "wronguser" } });
    fireEvent.change(passwordInput, { target: { value: "wrongpass" } });
    fireEvent.click(submitButton);

    await waitFor(() => {
      expect(
        screen.getByText("Invalid username or password"),
      ).toBeInTheDocument();
    });
  });

  it("calls login function with correct credentials", async () => {
    const mockUser = {
      id: "1",
      username: "admin",
      role: "admin",
      created_at: Date.now(),
    };

    vi.mocked(api.get).mockRejectedValue(new Error("Not authenticated"));
    vi.mocked(api.post).mockResolvedValue({
      data: { user: mockUser, message: "Login successful" },
    });

    renderLogin();

    const usernameInput = screen.getByLabelText("Username");
    const passwordInput = screen.getByLabelText("Password");
    const submitButton = screen.getByRole("button", { name: /sign in/i });

    fireEvent.change(usernameInput, { target: { value: "admin" } });
    fireEvent.change(passwordInput, { target: { value: "admin" } });
    fireEvent.click(submitButton);

    await waitFor(() => {
      expect(api.post).toHaveBeenCalledWith("/auth/login", {
        username: "admin",
        password: "admin",
      });
    });
  });

  it("requires both username and password", () => {
    renderLogin();

    const usernameInput = screen.getByLabelText("Username");
    const passwordInput = screen.getByLabelText("Password");

    expect(usernameInput).toHaveAttribute("required");
    expect(passwordInput).toHaveAttribute("required");
  });

  it("password field has type password", () => {
    renderLogin();

    const passwordInput = screen.getByLabelText("Password");
    expect(passwordInput).toHaveAttribute("type", "password");
  });
});
