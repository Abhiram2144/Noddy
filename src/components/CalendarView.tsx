import React from "react";
import { Calendar, Clock, MapPin } from "lucide-react";

interface CalendarEvent {
  id: string;
  subject: string;
  start: string;
  end: string;
  location: string;
  is_all_day: boolean;
}

interface CalendarViewProps {
  events: CalendarEvent[];
  title?: string;
  provider?: string;
  metadata?: {
    is_demo?: boolean;
  };
}

const CalendarView: React.FC<CalendarViewProps> = ({
  events,
  title,
  provider,
  metadata,
}) => {
  const formatDate = (dateString: string) => {
    const date = new Date(dateString);
    return date.toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" });
  };

  const displayProvider =
    provider === "google" ? "Google Calendar" : "Outlook Calendar";
  const displayTitle = title || `Your ${displayProvider} Schedule`;

  return (
    <div className="calendar-view bg-glass p-4 rounded-xl border border-white/10 shadow-xl max-w-md w-full animate-fade-in mb-4">
      <div className="flex items-center justify-between mb-4">
        <div className="flex items-center gap-2 text-primary font-bold">
          <Calendar className="w-5 h-5" />
          <h3 className="text-lg">{displayTitle}</h3>
        </div>
        {metadata?.is_demo && (
          <span className="bg-amber-500/20 text-amber-500 text-[10px] px-2 py-0.5 rounded-full border border-amber-500/30 font-bold uppercase tracking-wider">
            Demo Mode
          </span>
        )}
      </div>

      {events.length === 0 ? (
        <p className="text-white/60 text-center py-4">
          No events found for this period.
        </p>
      ) : (
        <div className="space-y-3">
          {events.map((event) => (
            <div
              key={event.id}
              className="event-card bg-white/5 p-3 rounded-lg border border-white/5 hover:border-primary/30 transition-all duration-300"
            >
              <h4 className="font-semibold text-white mb-1">{event.subject}</h4>
              <div className="flex flex-wrap gap-x-4 gap-y-1 text-sm text-white/70">
                <div className="flex items-center gap-1">
                  <Clock className="w-3.5 h-3.5 text-primary" />
                  <span>
                    {formatDate(event.start)} - {formatDate(event.end)}
                  </span>
                </div>
                {event.location && (
                  <div className="flex items-center gap-1">
                    <MapPin className="w-3.5 h-3.5 text-primary" />
                    <span>{event.location}</span>
                  </div>
                )}
              </div>
            </div>
          ))}
        </div>
      )}

      <div className="mt-4 pt-3 border-t border-white/5 flex justify-between items-center text-xs text-white/40">
        <span>Source: {displayProvider}</span>
        <span className="bg-primary/20 text-primary-light px-2 py-0.5 rounded-full">
          Synced
        </span>
      </div>
    </div>
  );
};

export default CalendarView;
