import { useState } from 'react';
import { Settings as SettingsIcon, Users, Database, Bell } from 'lucide-react';
import UserList from '../admin/UserList';

export function Settings() {
  const [activeTab, setActiveTab] = useState<'general' | 'backends' | 'users' | 'notifications'>('general');

  const tabs = [
    { id: 'general', label: 'General', icon: SettingsIcon },
    { id: 'backends', label: 'Backends', icon: Database },
    { id: 'users', label: 'Users', icon: Users },
    { id: 'notifications', label: 'Notifications', icon: Bell },
  ];

  return (
    <div className="p-6">
      <h1 className="text-slate-900 mb-6">Settings</h1>

      {/* Tabs */}
      <div className="bg-white rounded-lg border border-slate-200 overflow-hidden">
        <div className="border-b border-slate-200">
          <div className="flex">
            {tabs.map((tab) => {
              const Icon = tab.icon;
              return (
                <button
                  key={tab.id}
                  onClick={() => setActiveTab(tab.id as any)}
                  className={`flex items-center gap-2 px-6 py-4 text-sm font-medium border-b-2 transition-colors ${
                    activeTab === tab.id
                      ? 'border-blue-600 text-blue-600'
                      : 'border-transparent text-slate-600 hover:text-slate-900'
                  }`}
                >
                  <Icon className="w-4 h-4" />
                  {tab.label}
                </button>
              );
            })}
          </div>
        </div>

        {/* Tab Content */}
        <div className="p-6">
          {activeTab === 'general' && (
            <div className="space-y-6">
              <div>
                <h3 className="text-lg font-medium text-slate-900 mb-4">General Settings</h3>
                <div className="space-y-4">
                  <div className="flex items-center justify-between py-3 border-b border-slate-200">
                    <div>
                      <div className="font-medium text-slate-900">Theme</div>
                      <div className="text-sm text-slate-600">Choose your preferred color scheme</div>
                    </div>
                    <select className="px-3 py-2 border border-slate-300 rounded-lg">
                      <option>Light</option>
                      <option>Dark (Coming Soon)</option>
                      <option>System</option>
                    </select>
                  </div>

                  <div className="flex items-center justify-between py-3 border-b border-slate-200">
                    <div>
                      <div className="font-medium text-slate-900">Default Workers</div>
                      <div className="text-sm text-slate-600">Default number of parallel workers for new jobs</div>
                    </div>
                    <input
                      type="number"
                      defaultValue="4"
                      min="1"
                      max="16"
                      className="w-20 px-3 py-2 border border-slate-300 rounded-lg"
                    />
                  </div>
                </div>
              </div>
            </div>
          )}

          {activeTab === 'backends' && (
            <div className="space-y-6">
              <div className="flex items-center justify-between">
                <h3 className="text-lg font-medium text-slate-900">Backend Configuration</h3>
                <button className="px-4 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700">
                  Add Backend
                </button>
              </div>
              <div className="text-center py-12 border-2 border-dashed border-slate-200 rounded-lg">
                <Database className="w-12 h-12 text-slate-400 mx-auto mb-2" />
                <p className="text-slate-600">No backends configured</p>
                <p className="text-sm text-slate-500 mt-1">Add S3, SMB, or SSH backends to get started</p>
              </div>
            </div>
          )}

          {activeTab === 'users' && (
            <div>
              <UserList />
            </div>
          )}

          {activeTab === 'notifications' && (
            <div className="space-y-6">
              <h3 className="text-lg font-medium text-slate-900 mb-4">Notification Preferences</h3>
              <div className="space-y-4">
                <div className="flex items-center justify-between py-3 border-b border-slate-200">
                  <div>
                    <div className="font-medium text-slate-900">Job Completion</div>
                    <div className="text-sm text-slate-600">Notify when jobs finish</div>
                  </div>
                  <input type="checkbox" defaultChecked className="w-4 h-4" />
                </div>

                <div className="flex items-center justify-between py-3 border-b border-slate-200">
                  <div>
                    <div className="font-medium text-slate-900">Job Failures</div>
                    <div className="text-sm text-slate-600">Alert on job errors</div>
                  </div>
                  <input type="checkbox" defaultChecked className="w-4 h-4" />
                </div>
              </div>
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
