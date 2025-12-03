import { useState } from 'react';
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { api } from '../../lib/api';

interface User {
  id: string;
  username: string;
  role: string;
  created_at: number;
}

export default function UserList() {
  const queryClient = useQueryClient();
  const [isAdding, setIsAdding] = useState(false);
  const [newUser, setNewUser] = useState({ username: '', password: '', role: 'operator' });

  const { data: users, isLoading } = useQuery({
    queryKey: ['users'],
    queryFn: () => api.get<User[]>('/admin/users').then(r => r.data)
  });

  const createUser = useMutation({
    mutationFn: (data: typeof newUser) => api.post('/admin/users', data),
    onSuccess: () => {
      setIsAdding(false);
      queryClient.invalidateQueries({ queryKey: ['users'] });
      setNewUser({ username: '', password: '', role: 'operator' });
    }
  });

  if (isLoading) return <div className="p-4">Loading Users...</div>;

  return (
    <div className="space-y-4">
      <div className="flex justify-between items-center">
        <h2 className="text-xl font-bold flex items-center gap-2">
          <svg className="w-5 h-5 text-blue-600" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M9 12l2 2 4-4m5.618-4.016A11.955 11.955 0 0112 2.944a11.955 11.955 0 01-8.618 3.04A12.02 12.02 0 003 9c0 5.591 3.824 10.29 9 11.622 5.176-1.332 9-6.03 9-11.622 0-1.042-.133-2.052-.382-3.016z" />
          </svg>
          Team Management
        </h2>
        <button
          onClick={() => setIsAdding(!isAdding)}
          className="px-4 py-2 bg-blue-600 text-white rounded hover:bg-blue-700 flex items-center gap-2"
        >
          <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M18 9v3m0 0v3m0-3h3m-3 0h-3m-2-5a4 4 0 11-8 0 4 4 0 018 0zM3 20a6 6 0 0112 0v1H3v-1z" />
          </svg>
          Add User
        </button>
      </div>

      {isAdding && (
        <div className="bg-slate-50 p-4 rounded border flex gap-4 items-end">
          <input
            placeholder="Username"
            className="border p-2 rounded flex-1"
            value={newUser.username}
            onChange={e => setNewUser({...newUser, username: e.target.value})}
          />
          <input
            placeholder="Password"
            type="password"
            className="border p-2 rounded flex-1"
            value={newUser.password}
            onChange={e => setNewUser({...newUser, password: e.target.value})}
          />
          <select
            className="border p-2 rounded"
            value={newUser.role}
            onChange={e => setNewUser({...newUser, role: e.target.value})}
          >
            <option value="admin">Admin</option>
            <option value="operator">Operator</option>
            <option value="viewer">Viewer</option>
          </select>
          <button
            onClick={() => createUser.mutate(newUser)}
            className="px-4 py-2 bg-green-600 text-white rounded hover:bg-green-700"
          >
            Save
          </button>
        </div>
      )}

      <div className="border rounded-lg overflow-hidden bg-white">
        <table className="w-full text-sm text-left">
          <thead className="bg-gray-100 border-b">
            <tr>
              <th className="p-3">Username</th>
              <th className="p-3">Role</th>
              <th className="p-3">Created</th>
              <th className="p-3 text-right">Actions</th>
            </tr>
          </thead>
          <tbody>
            {users?.map(user => (
              <tr key={user.id} className="border-b hover:bg-gray-50">
                <td className="p-3 font-medium">{user.username}</td>
                <td className="p-3">
                  <span className={`px-2 py-1 rounded-full text-xs ${
                    user.role === 'admin' ? 'bg-purple-100 text-purple-800' : 'bg-blue-100 text-blue-800'
                  }`}>
                    {user.role}
                  </span>
                </td>
                <td className="p-3 text-gray-500">
                  {new Date(user.created_at * 1000).toLocaleDateString()}
                </td>
                <td className="p-3 text-right">
                  <button className="text-red-500 hover:text-red-700">
                    <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16" />
                    </svg>
                  </button>
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </div>
  );
}
