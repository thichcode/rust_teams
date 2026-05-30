//! Meeting detection JavaScript injection
//! Monitors Teams window for meeting indicators

/// JavaScript to detect meeting state changes
pub fn get_meeting_detection_script() -> String {
    r#"
    (function() {
        'use strict';
        
        console.log('[MeetingDetect] Initializing meeting detection...');
        
        let isMeetingActive = false;
        let meetingStartTime = null;
        
        // Check if we're in a meeting
        function checkMeetingState() {
            // Look for meeting indicators in Teams
            const meetingIndicators = [
                // Call/Meeting controls
                '[data-tid="call-controls"]',
                '[class*="call-controls"]',
                '[class*="meeting-controls"]',
                '[class*="audio-controls"]',
                '[class*="video-controls"]',
                
                // Call status
                '[class*="call-state"]',
                '[class*="meeting-state"]',
                
                // Active call banner
                '[class*="active-call"]',
                '[class*="ongoing-call"]'
            ];
            
            let meetingDetected = false;
            
            for (const selector of meetingIndicators) {
                const elements = document.querySelectorAll(selector);
                if (elements.length > 0) {
                    meetingDetected = true;
                    break;
                }
            }
            
            // Also check page title
            const title = document.title.toLowerCase();
            const titleIndicators = ['meeting', 'call', 'in a call', 'presenting', 'sharing'];
            
            for (const indicator of titleIndicators) {
                if (title.includes(indicator)) {
                    meetingDetected = true;
                    break;
                }
            }
            
            // Check URL for meeting
            const url = window.location.href.toLowerCase();
            if (url.includes('/meet/') || url.includes('/call/')) {
                meetingDetected = true;
            }
            
            // Update state
            if (meetingDetected && !isMeetingActive) {
                isMeetingActive = true;
                meetingStartTime = new Date();
                console.log('[MeetingDetect] Meeting started at', meetingStartTime.toISOString());
                
                // Notify parent window
                window.postMessage({
                    type: 'MEETING_STATE_CHANGED',
                    active: true,
                    startTime: meetingStartTime.toISOString()
                }, '*');
                
            } else if (!meetingDetected && isMeetingActive) {
                isMeetingActive = false;
                const duration = meetingStartTime ? 
                    Math.round((new Date() - meetingStartTime) / 1000) : 0;
                console.log('[MeetingDetect] Meeting ended, duration:', duration, 'seconds');
                
                // Notify parent window
                window.postMessage({
                    type: 'MEETING_STATE_CHANGED',
                    active: false,
                    duration: duration
                }, '*');
                
                meetingStartTime = null;
            }
        }
        
        // Initial check
        checkMeetingState();
        
        // Check periodically
        setInterval(checkMeetingState, 2000);
        
        // Check on DOM changes
        const observer = new MutationObserver(() => {
            checkMeetingState();
        });
        
        if (document.body) {
            observer.observe(document.body, {
                childList: true,
                subtree: true
            });
        } else {
            document.addEventListener('DOMContentLoaded', () => {
                observer.observe(document.body, {
                    childList: true,
                    subtree: true
                });
            });
        }
        
        console.log('[MeetingDetect] ✓ Meeting detection enabled');
        
    })();
    "#
    .to_string()
}
