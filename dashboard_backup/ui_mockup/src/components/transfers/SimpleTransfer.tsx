import { LocationSelector } from './LocationSelector';
import { ControlBar } from './ControlBar';
import { StatusLog } from './StatusLog';
import { ArrowRight } from 'lucide-react';

export function SimpleTransfer() {
  return (
    <div className="space-y-6">
      {/* Transfer Panel */}
      <div className="bg-white rounded-xl shadow-sm border border-slate-200 p-6">
        <div className="grid grid-cols-1 lg:grid-cols-[1fr,auto,1fr] gap-6 items-center">
          <LocationSelector 
            title="Source" 
            type="source"
          />
          
          <div className="flex justify-center">
            <div className="w-12 h-12 bg-blue-50 rounded-full flex items-center justify-center">
              <ArrowRight className="w-6 h-6 text-blue-600" />
            </div>
          </div>
          
          <LocationSelector 
            title="Destination" 
            type="destination"
          />
        </div>
      </div>

      <ControlBar />
      <StatusLog />
    </div>
  );
}
