//! Meeting detection JavaScript injection
//! Monitors Teams window for meeting indicators and sends IPC messages

/// JavaScript to detect meeting state changes and notify Rust backend
pub fn get_meeting_detection_script() -> String {
    r#"
    (function() {
        'use strict';
        
        console.log('[MeetingDetect] Initializing meeting detection...');
        
        let isMeetingActive = false;
        let meetingStartTime = null;
        
        // Send message to Rust backend via IPC
        function notifyBackend(type, data) {
            if (window.__TAURI_IPC__) {
                window.__TAURI_IPC__.invoke(type, data);
            } else if (window.chrome?.webview?.postMessage) {
                window.chrome.webview.postMessage(JSON.stringify({type, data}));
            }
        }
        
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
                
                // Notify Rust backend via IPC
                notifyBackend('meeting_state_changed', {
                    active: true,
                    startTime: meetingStartTime.toISOString()
                });
                
            } else if (!meetingDetected && isMeetingActive) {
                isMeetingActive = false;
                const duration = meetingStartTime ? 
                    Math.round((new Date() - meetingStartTime) / 1000) : 0;
                console.log('[MeetingDetect] Meeting ended, duration:', duration, 'seconds');
                
                // Notify Rust backend via IPC
                notifyBackend('meeting_state_changed', {
                    active: false,
                    duration: duration,
                    startTime: meetingStartTime?.toISOString()
                });
                
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
