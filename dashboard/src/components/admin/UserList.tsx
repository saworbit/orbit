import { useState } from "react";
import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { api } from "../../lib/api";
import { UserPlus, Shield, Trash2, Calendar, Users } from "lucide-react";

interface User {
  id: string;
  username: string;
  role: string;
  created_at: number;
}

export default function UserList() {
  const queryClient = useQueryClient();
  const [isAdding, setIsAdding] = useState(false);
  const [newUser, setNewUser] = useState({
    username: "",
    password: "",
    role: "operator",
  });

  const { data: users, isLoading } = useQuery({
    queryKey: ["users"],
    queryFn: () => api.get<User[]>("/admin/users").then((r) => r.data),
  });

  const createUser = useMutation({
    mutationFn: (data: typeof newUser) => api.post("/admin/users", data),
    onSuccess: () => {
      setIsAdding(false);
      queryClient.invalidateQueries({ queryKey: ["users"] });
      setNewUser({ username: "", password: "", role: "operator" });
    },
  });

  const deleteUser = useMutation({
    mutationFn: (userId: string) => api.delete(`/admin/users/${userId}`),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["users"] });
    },
  });

  if (isLoading) {
    return (
      <div className="w-full h-48 flex items-center justify-center text-muted-foreground animate-pulse">
        Loading users...
      </div>
    );
  }

  const getRoleBadge = (role: string) => {
    const styles: Record<string, string> = {
      admin: "bg-purple-100 text-purple-800 dark:bg-purple-900/30 dark:text-purple-400 border-purple-200 dark:border-purple-800",
      operator: "bg-blue-100 text-blue-800 dark:bg-blue-900/30 dark:text-blue-400 border-blue-200 dark:border-blue-800",
      viewer: "bg-gray-100 text-gray-800 dark:bg-gray-800 dark:text-gray-400 border-gray-200 dark:border-gray-700",
    };
    return (
      <span className={`px-2.5 py-0.5 rounded-full text-xs font-bold border capitalize ${styles[role.toLowerCase()] || styles.viewer}`}>
        {role}
      </span>
    );
  };

  return (
    <div className="space-y-6">
      {/* Header */}
      <div className="flex flex-col sm:flex-row justify-between items-start sm:items-center gap-4">
        <div>
          <h2 className="text-3xl font-bold tracking-tight flex items-center gap-3">
            <Shield className="text-primary" size={32} />
            Team Management
          </h2>
          <p className="text-muted-foreground mt-1">Manage user accounts and permissions</p>
        </div>
        <button
          onClick={() => setIsAdding(!isAdding)}
          className="flex items-center gap-2 px-4 py-2.5 bg-primary text-primary-foreground rounded-lg font-medium hover:bg-primary/90 shadow-lg shadow-primary/20 transition-all"
        >
          <UserPlus size={18} />
          Add User
        </button>
      </div>

      {/* Add User Form */}
      {isAdding && (
        <div className="bg-card border rounded-xl p-6 shadow-sm space-y-4">
          <h3 className="font-semibold flex items-center gap-2">
            <UserPlus size={18} className="text-primary" />
            Create New User
          </h3>
          <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
            <div className="space-y-2">
              <label className="text-sm font-medium text-muted-foreground">Username</label>
              <input
                placeholder="Enter username"
                className="w-full px-3 py-2 bg-background border rounded-md focus:outline-none focus:ring-2 focus:ring-primary"
                value={newUser.username}
                onChange={(e) => setNewUser({ ...newUser, username: e.target.value })}
              />
            </div>
            <div className="space-y-2">
              <label className="text-sm font-medium text-muted-foreground">Password</label>
              <input
                placeholder="Enter password"
                type="password"
                className="w-full px-3 py-2 bg-background border rounded-md focus:outline-none focus:ring-2 focus:ring-primary"
                value={newUser.password}
                onChange={(e) => setNewUser({ ...newUser, password: e.target.value })}
              />
            </div>
            <div className="space-y-2">
              <label className="text-sm font-medium text-muted-foreground">Role</label>
              <select
                className="w-full px-3 py-2 bg-background border rounded-md focus:outline-none focus:ring-2 focus:ring-primary cursor-pointer"
                value={newUser.role}
                onChange={(e) => setNewUser({ ...newUser, role: e.target.value })}
              >
                <option value="admin">Admin</option>
                <option value="operator">Operator</option>
                <option value="viewer">Viewer</option>
              </select>
            </div>
          </div>
          <div className="flex justify-end gap-2">
            <button
              onClick={() => setIsAdding(false)}
              className="px-4 py-2 border rounded-lg hover:bg-accent transition-colors"
            >
              Cancel
            </button>
            <button
              onClick={() => createUser.mutate(newUser)}
              disabled={!newUser.username || !newUser.password}
              className="px-4 py-2 bg-green-600 text-white rounded-lg hover:bg-green-700 disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
            >
              Create User
            </button>
          </div>
        </div>
      )}

      {/* User Stats */}
      <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
        <div className="bg-card border rounded-xl p-4">
          <div className="flex items-center gap-3">
            <div className="p-3 bg-blue-500/10 rounded-lg">
              <Users size={24} className="text-blue-500" />
            </div>
            <div>
              <p className="text-sm text-muted-foreground">Total Users</p>
              <p className="text-2xl font-bold">{users?.length || 0}</p>
            </div>
          </div>
        </div>
        <div className="bg-card border rounded-xl p-4">
          <div className="flex items-center gap-3">
            <div className="p-3 bg-purple-500/10 rounded-lg">
              <Shield size={24} className="text-purple-500" />
            </div>
            <div>
              <p className="text-sm text-muted-foreground">Administrators</p>
              <p className="text-2xl font-bold">
                {users?.filter((u) => u.role === "admin").length || 0}
              </p>
            </div>
          </div>
        </div>
        <div className="bg-card border rounded-xl p-4">
          <div className="flex items-center gap-3">
            <div className="p-3 bg-green-500/10 rounded-lg">
              <Users size={24} className="text-green-500" />
            </div>
            <div>
              <p className="text-sm text-muted-foreground">Operators</p>
              <p className="text-2xl font-bold">
                {users?.filter((u) => u.role === "operator").length || 0}
              </p>
            </div>
          </div>
        </div>
      </div>

      {/* Users Table */}
      <div className="bg-card border rounded-xl shadow-sm overflow-hidden">
        {users && users.length === 0 ? (
          <div className="p-12 text-center">
            <div className="w-16 h-16 bg-muted rounded-full flex items-center justify-center mx-auto mb-4">
              <Users className="text-muted-foreground" size={32} />
            </div>
            <h3 className="text-lg font-medium">No users found</h3>
            <p className="text-muted-foreground">Create your first user to get started</p>
          </div>
        ) : (
          <div className="overflow-x-auto">
            <table className="w-full">
              <thead className="bg-muted/50 border-b border-border">
                <tr>
                  <th className="text-left p-4 text-sm font-semibold text-muted-foreground">
                    Username
                  </th>
                  <th className="text-left p-4 text-sm font-semibold text-muted-foreground">
                    Role
                  </th>
                  <th className="text-left p-4 text-sm font-semibold text-muted-foreground">
                    Created
                  </th>
                  <th className="text-right p-4 text-sm font-semibold text-muted-foreground">
                    Actions
                  </th>
                </tr>
              </thead>
              <tbody className="divide-y divide-border">
                {users?.map((user) => (
                  <tr key={user.id} className="hover:bg-accent/50 transition-colors">
                    <td className="p-4">
                      <div className="flex items-center gap-3">
                        <div className="w-8 h-8 rounded-full bg-gradient-to-tr from-blue-500 to-purple-500 flex items-center justify-center text-white text-xs font-bold">
                          {user.username.substring(0, 2).toUpperCase()}
                        </div>
                        <span className="font-medium">{user.username}</span>
                      </div>
                    </td>
                    <td className="p-4">{getRoleBadge(user.role)}</td>
                    <td className="p-4">
                      <div className="flex items-center gap-2 text-sm text-muted-foreground">
                        <Calendar size={14} />
                        {new Date(user.created_at * 1000).toLocaleDateString()}
                      </div>
                    </td>
                    <td className="p-4 text-right">
                      <button
                        onClick={() => {
                          if (confirm(`Delete user "${user.username}"?`)) {
                            deleteUser.mutate(user.id);
                          }
                        }}
                        className="inline-flex items-center gap-1 px-3 py-1.5 text-red-500 hover:bg-red-500/10 rounded text-xs font-medium transition-colors"
                      >
                        <Trash2 size={14} />
                        Delete
                      </button>
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        )}
      </div>
    </div>
  );
}
